// Copyright 2021  The Hypatia Authors
// All rights reserved
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

#![feature(allocator_api)]
#![feature(sync_unsafe_cell)]
#![cfg_attr(not(test), no_main)]
#![cfg_attr(not(test), no_std)]
#![forbid(absolute_paths_not_starting_with_crate)]
#![forbid(elided_lifetimes_in_paths)]
#![forbid(unsafe_op_in_unsafe_fn)]

//! # Theon: System coldboot loader
//!
//! Theon is the coldboot loader, and has has two primary
//! responsibilities:
//!
//! 1. Taking an inventory of and initializing the hardware so
//!    that the system is in a known state, and so that a
//!    description of the machine can be passed when control is
//!    transferred to the supervisor.  For example, all
//!    processors will be parked in the supervisor, running in a
//!    fully-formed address space, etc.
//! 2. Loading and initializing all of the binaries that make
//!    up the system.  That is, the binary images of all
//!    segments and tasks will be loaded, and segment
//!    initializers will be invoked.
//!
//! Any failure at any point in this process is fatal, and boot
//! fails.  Hence, we use the infallible allocation interface
//! and make liberal use of panic!(), assert!(), and similar
//! interfaces.
//!
//! ## Binary loading
//!
//! Binary images are loaded into memory by an earlier stage
//! loader, and assumed to be resident once theon begins
//! execution.  Theon will locate them, and load them into
//! physical memory.
//!
//! Each binary is allocated a 16MiB region of physical RAM for
//! its various pages; these regions begin at 64MiB and are
//! aligned on 32MiB boundaries, giving us room for loading new
//! images into the second 16MiBs of each binary's region for
//! hitless update.
//!
//! Binaries represent either tasks or segments; see HDP 0002
//! for a high level overview of the distinction.  For segments,
//! the segment initializer will be invoked.  For tasks, the
//! task will be loaded and an image prototype transferred into
//! supervisor.
//!
//! ## Control Transfer
//!
//! Once Theon has initialized the system and loaded binaries,
//! it will transfer control into the supervisor segment,
//! passing a pointer to a serialized description of the system
//! and its state.  This is the same system description that is
//! given to the same entry path on hitless upgrade; thus, in
//! some ways one may think of Theon as synthesizing a hitless
//! update that starts the system running with no VMs.
//!
//! After theon has finished executing and transferred control
//! into the supervisor, it will not run again, and it's
//! resources --- in particular all memory associated with it,
//! including its image --- are reclaimed.
//!
//! ## Colophon
//!
//! In antiquity, Theon was Hypatia's father.

extern crate alloc;

mod allocator;
mod theon;
mod x86_64;

use alloc::vec::Vec;
use core::ops::Range;

use crate::x86_64::memory::{Region, Type};
use crate::x86_64::mp;
use arch::{HPA, MIB, PF4K, V4KA, VPageAddr};

type Result<T> = core::result::Result<T, &'static str>;

// Describes whether a given binary is a segment or a task,
// see HDPs 0002, 0009, and 0010 for details.
#[derive(Clone, Copy, Debug)]
enum BinaryType {
    Segment,
    Task,
}

/// Metadata used in the binary table: the name of the binary,
/// it's physical load address, and its type (either a segment
/// or a task).
type BinaryMeta = (&'static str, HPA, BinaryType);

/// Binaries are loaded in 16MiB regions of physical memory
/// that are aligned on 32MiB boundaries, starting at 64MiB.
const fn load_addr(offset: usize) -> HPA {
    let addr = (64 + offset * 32) * MIB;
    HPA::new(addr as u64)
}
const BINARY_IMAGE_MEMORY_SIZE: usize = 16 * MIB;

/// A table description all the binaries that are loaded by
/// theon, where to load them in physical memory, and their
/// type.
const BINARY_TABLE: &[BinaryMeta] = &[
    ("devices", load_addr(0), BinaryType::Segment),
    ("global", load_addr(1), BinaryType::Segment),
    ("memory", load_addr(2), BinaryType::Segment),
    ("monitor", load_addr(3), BinaryType::Segment),
    ("scheduler", load_addr(4), BinaryType::Segment),
    ("supervisor", load_addr(5), BinaryType::Segment),
    ("trace", load_addr(6), BinaryType::Segment),
    ("system", load_addr(7), BinaryType::Task),
    ("vcpu", load_addr(8), BinaryType::Task),
    ("vm", load_addr(9), BinaryType::Task),
];
const BINARY_LOAD_REGION_START: HPA = load_addr(0);
const BINARY_LOAD_REGION_END: HPA = load_addr(BINARY_TABLE.len());

/// Main entry point for the loader.
///
/// When we enter `main()`, the first 4GiB of the
/// physical address space are mapped R/W at
/// arch::HYPER_BASE_VADDR.  Note that the memory
/// regions that make up both the binary archive
/// as well as our load regions are mapped within
/// this region, so we can address them via pointers.
#[cfg_attr(not(test), unsafe(no_mangle))]
pub extern "C" fn main(mbinfo_phys: u64) -> ! {
    arch::lapic::enable_x2apic();
    let multiboot = x86_64::platform::init::start(mbinfo_phys);
    let crate::x86_64::pc::multiboot1::InitInfo { memory_regions, regions, modules } =
        multiboot.info();
    assert!(theon_fits(&regions));
    core::mem::drop(memory_regions);
    uart::panic_println!("end = {:016x?}", theon::end_addr());
    uart::panic_println!("regions: {:#x?}", regions);
    // TODO(cross): We really ought to clean this up.
    let bins = modules.iter().find(|&m| m.name == Some("bin.a")).expect("found 'bin.a' in modules");
    assert!(
        unsafe { bins.bytes.as_ptr().add(bins.bytes.len()) }.addr()
            < theon::vaddr(BINARY_LOAD_REGION_START).addr()
    );
    let archive = goblin::archive::Archive::parse(bins.bytes).expect("cannot parse bin.a");
    uart::panic_println!("Binary archive: {:#x?}", archive);
    clear_binary_load_region();
    for &(name, addr, typ) in BINARY_TABLE {
        let bytes = archive.extract(name, bins.bytes).expect("cannot extract elf");
        let region_end = addr.offset(BINARY_IMAGE_MEMORY_SIZE);
        load(name, typ, bytes, addr..region_end).expect("loaded binary");
    }
    unsafe { core::arch::asm!("int3") };
    // Start other CPUs.
    uart::panic_println!("starting APs");
    unsafe {
        mp::start_aps(cpus());
    }
    panic!("main: trapstubs = {:#x?}", arch::trap::stubs as usize);
}

// XXX: This is temporary, for testing purposes only.
//
// TODO(cross): We need to extract the list of CPUs from
// somewhere, such as the ACPI MADT on the PC platform, or from
// AMD platform-specific config on the Oxide architecture.  We
// should also allocate stacks for the CPUs from memory that is
// close to them (e.g., in the same NUMA domain or subdomain).
fn cpus() -> &'static [mp::EntryCPU] {
    fn stack() -> usize {
        const NPAGES: usize = 8;
        const STACK_SIZE: usize = core::mem::size_of::<arch::Page4K>() * NPAGES;
        #[cfg(not(test))]
        use alloc::boxed::Box;
        let s = Box::new([const { arch::Page4K::new() }; NPAGES]);
        let stack = &s[0];
        let ptr = stack as *const arch::Page4K as *const u8;
        let top = unsafe { ptr.add(STACK_SIZE) };
        Box::leak(s);
        top.addr()
    }
    let cs = alloc::vec![
        mp::EntryCPU::new(arch::ProcessorID(0), stack()),
        mp::EntryCPU::new(arch::ProcessorID(1), stack()),
        mp::EntryCPU::new(arch::ProcessorID(2), stack()),
        mp::EntryCPU::new(arch::ProcessorID(3), stack()),
    ];
    cs.leak()
}

fn theon_fits(regions: &[Region]) -> bool {
    assert!(theon::end_addr().addr() < theon::vaddr(BINARY_LOAD_REGION_START).addr());
    for region in regions.iter().filter(|&r| r.typ == Type::RAM) {
        if region.start <= BINARY_LOAD_REGION_START.addr()
            && BINARY_LOAD_REGION_END.addr() <= region.end
        {
            return true;
        }
    }
    false
}

/// Zeroes the memory region that binaries are loaded into.
fn clear_binary_load_region() {
    let start = theon::vaddr(BINARY_LOAD_REGION_START);
    let end = theon::vaddr(BINARY_LOAD_REGION_END);
    unsafe { core::ptr::write_bytes(start.cast_mut(), 0, end.offset_from_unsigned(start)) };
}

/// Loads the named binary of the given type into given physical region.
fn load(name: &str, typ: BinaryType, bytes: &[u8], region: Range<HPA>) -> Result<PF4K> {
    use arch::{Page, Page4K};
    let elf = goblin::elf::Elf::parse(bytes).expect("cannot parse elf");
    uart::panic_println!(
        "ELF for {:#?} ({:?}@{:x?}): {:#x?}",
        name,
        typ,
        region,
        elf.program_headers
    );
    let mut regions = Vec::new();
    let mut headers = Vec::new();
    for header in
        elf.program_headers.iter().filter(|h| h.p_type == goblin::elf::program_header::PT_LOAD)
    {
        let vm = header.vm_range();
        // All Hypatia binaries require that loadable sections
        // are aligned on 4KiB boundaries.
        assert_eq!(vm.start % 4096, 0);
        assert!(vm.start < vm.end);
        regions.push(V4KA::new(vm.start)..V4KA::new_round_up(vm.end));
        headers.push(header);
    }
    let base = theon::vaddr(region.start).cast_mut();
    let len = unsafe { theon::vaddr(region.end).offset_from_unsigned(theon::vaddr(region.start)) };
    let heap = unsafe { allocator::Block::new_from_raw_parts(base, len) };
    let bump = allocator::BumpAlloc::new(heap);
    let allocate = || {
        use alloc::alloc::Allocator;
        let layout = alloc::alloc::Layout::new::<Page4K>();
        let mem = bump.allocate(layout).map_err(|_| "failed to allocatoe page")?;
        let page = unsafe { &mut *Page4K::proto_ptr().with_addr(mem.addr().into()).cast_mut() };
        Ok(page)
    };
    let root = allocate().expect("allocated root page for binary");
    let root = arch::vm::make_shared_ranges(&regions, root.frame(), &mut || {
        let page = allocate()?;
        Ok(page.frame())
    })
    .expect("mapped mem regions");
    for (&header, region) in headers.iter().zip(&regions) {
        let mut src = &bytes[header.file_range()];
        let r = header.is_read();
        let w = header.is_write();
        let x = header.is_executable();

        for addr in region.clone() {
            let page = allocate().expect("allocated data page");
            if !src.is_empty() {
                let len = usize::min(src.len(), Page4K::SIZE);
                let dst = theon::VZERO.with_addr(page.vaddr().addr()).cast_mut();
                unsafe {
                    core::ptr::copy_nonoverlapping(src.as_ptr(), dst, len);
                }
                src = &src[len..];
            }
            arch::vm::map_leaf(page.frame(), addr, r, w, x).expect("mapped a page");
        }
    }
    if let BinaryType::Task = typ {
        arch::vm::unmap_root_ranges(&regions);
    } else {
        let entry = elf.entry as usize;
        let init = unsafe { core::mem::transmute::<usize, fn()>(entry) };
        init();
    }
    Ok(root)
}

#[cfg_attr(test, allow(dead_code))]
#[unsafe(no_mangle)]
pub extern "C" fn apmain(cpu: arch::ProcessorID) -> ! {
    use core::sync::atomic::{AtomicBool, Ordering};
    static S: AtomicBool = AtomicBool::new(false);
    while S.compare_exchange(false, true, Ordering::SeqCst, Ordering::Acquire).is_err() {
        arch::cpu::relax();
    }
    uart::panic_println!("Hello from {}", u32::from(cpu));
    S.store(false, Ordering::Release);
    mp::signal_ap(cpu);
    unsafe {
        core::arch::asm!("hlt");
    }
    panic!("apmain");
}

hypatia::runtime!();
