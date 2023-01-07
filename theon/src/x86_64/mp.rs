// Copyright 2022  The Hypatia Authors
// All rights reserved
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! Theon is responsible for starting the APs and getting them
//! parked in the supervisor, pending the rest of system
//! startup.  This code handles the low-level bootstrapping
//! details.
//!
//! The x86 startup sequence is a bit unusual, in that at reset
//! or power-on, a particular CPU is elected to be the first
//! processor to start, called the bootstrap processor (BSP) by
//! Intel or bootstrap core (BSC) by AMD, while the others are
//! designated Application Processors (APs) and enter a state
//! where they wait to be started by the first.
//!
//! The sequence to start an AP is for the BSP to send an INIT
//! IPI to the AP, wait 10ms, and then send two startup IPIs
//! (SIPIs) to the AP, each separated by 200us.  The need to
//! send two SIPIs is a workaround for a long-fixed
//! architectural bug, but is enshrined in the hardware
//! specification and endures in software.
//!
//! Receipt of an INIT IPI puts a CPU into an architecturally
//! well-defined state where it is waiting for a SIPI.  The
//! behavior of a CPU that is in the INIT state awaiting a
//! startup IPI is similarly well-defined.  When such a
//! processor receives a SIPI, it will begin executing in 16-bit
//! real mode, at a location derived from the SIPI vector
//! number: The vector number will be translated to a 4KiB page
//! frame, the real mode IP will be 0, and the code and data
//! segments will be set to the vector page address.  That is,
//! the CPU will start executing at SIPI_VECTOR * 4096.
//!
//! The code located at the page defined by the SIPI vector is
//! position-independent and will transition the AP into long
//! mode with paging enabled and then jump into theon.

use crate::theon;
use core::sync::atomic::{AtomicU32, Ordering};
use core::time::Duration;

/// Describes CPUs and their stacks as known to the system for
/// early startup.
///
/// Theon on the BSP is responsible for creating an array of
/// these, one for each CPU in the system, and allocating stacks
/// for those CPUs to boot on.  That array is made known to
/// assembler code by embedding a pointer to it and its length
/// in the SIPI page; we put both data at the end of the page
/// for easy location in assembler.  The AP bootstrap code will
/// iterate through that list, matching its own LAPIC ID against
/// those in the array.  Once it has found its own entry, it
/// knows its own CPU number (distinct from APIC IDs, which need
/// not be contiguous) and can use the given stack to jump into
/// Rust code.
///
/// The `state` field is used in early assembler code to
/// indicate whether an AP is executing or not.  If it is,
/// the low bit will be set.
///
/// Shared with assembler.
#[derive(Debug)]
#[repr(C)]
pub struct EntryCPU {
    apic_id: arch::ProcessorID,
    state: AtomicU32,
    stack: usize,
}
static_assertions::const_assert_eq!(core::mem::size_of::<EntryCPU>(), 16);

impl EntryCPU {
    pub fn new(apic_id: arch::ProcessorID, stack: usize) -> EntryCPU {
        let state = AtomicU32::new(0);
        EntryCPU { apic_id, state, stack }
    }
}

/// The vector we use for startup IPIs.
///
/// On receipt of a SIPI, an AP will start at the page number of
/// this vector (that is, page 7, or 0x7000).  The choice here
/// is mostly arbitrary; since there are only 256 vectors, we
/// are constrained to starting at some location in the first
/// 1MiB of the physical address space.
///
/// We choose 7 because we know that, on PC-style
/// configurations, there is RAM there and it is unlikely to
/// conflict with e.g. legacy devices or data areas used by
/// firmware.
const SIPI_VECTOR: u8 = 7;

/// Start the APs.
pub unsafe fn start_aps(cpus: &'static [EntryCPU]) {
    setup_sipi_page(cpus);
    unsafe {
        init_sipi_sipi(cpus);
    }
    wait_for_aps(cpus);
}

// Set up the SIPI vector page.
//
// As described above, an INIT'd AP will start executing at
// SIPI_VECTOR * 4096 on receipt of a SIPI.  This code is copies
// the position-independent early AP startup code to that
// address, and makes sure that the length of and a pointer to
// the CPU list are embedded in the SIPI page so that they may
// be found by assembly code.
//
// SIPI_VECTOR is defined to be 7, so the layout of the SIPI
// page after this is called looks something like this:
//
//                    +------------------------------------- +
// 0x7000             | AP startup code...                   |
//                    | ...                                  |
// 0x7xxx             | End of AP startup code               |
//                    |   .                                  |
//                    |   . (unused)                         |
//                    |   .                                  |
// 0x7000 + 4096 - 16 | Length of CPUs                       |
// 0x7000 + 4096 -  8 | Pointer to CPUs......................|
// 0x8000             +--------------------------------------+
//
// It is expected that this code is only called once, but it is
// idempotent, so that is not enforced.
fn setup_sipi_page(cpus: &'static [EntryCPU]) {
    let pa = arch::HPA::new(u64::from(SIPI_VECTOR) * 4096);
    let va = theon::vaddr(pa);
    let dst = va as *mut u8;
    let apstart = theon::apstart();
    unsafe {
        // The startup code at the beginning of the SIPI page is
        // position independent, so we just copy it into place.
        core::ptr::copy_nonoverlapping(apstart.as_ptr(), dst, apstart.len());
        // The pointer to the EntryCPU vector and its length are
        // written to the last and next-to-last word of the page,
        // respectively.
        let sipi_page_end = dst.add(4096) as *mut usize;
        let cpus_ptr = sipi_page_end.sub(1);
        let ncpus_ptr = sipi_page_end.sub(2);
        core::ptr::write(cpus_ptr, cpus.as_ptr().addr());
        core::ptr::write(ncpus_ptr, cpus.len());
    }
}

// Send the interprocessor startup interrupt sequence.
//
// This uses broadcast IPIs for the INIT and first STARTUP IPI
// so that all APs start approximately simultaneously.  The APs
// will set a flag in the `state` field of their `EarlyCPU`
// indicating that they are running after the receipt of the
// SIPI; we probe that here to determine whether to send a
// second SIPI to individual processors.
const STATE_RUNNING: u32 = 1;
unsafe fn init_sipi_sipi(cpus: &'static [EntryCPU]) {
    // Send the INIT and first SIPI by broadcast IPIs
    // ("all-but-self") with a 10ms delay in between, as per the
    // Intel SDM.
    unsafe {
        arch::lapic::send_broadcast_init();
    }
    arch::cpu::pause(Duration::from_millis(10));
    unsafe {
        arch::lapic::send_broadcast_sipi(SIPI_VECTOR);
    }
    // For the next 200us, probe the state of all CPUs: if
    // they are all running, we're done.
    for _delay in 0..200 {
        if cpus.iter().all(|cpu| cpu.state.load(Ordering::SeqCst) == STATE_RUNNING) {
            return;
        }
        arch::cpu::pause(Duration::from_micros(1));
    }
    // Send a second SIPI to any CPUs that are not yet running.
    for cpu in cpus {
        if cpu.state.load(Ordering::SeqCst) != STATE_RUNNING {
            unsafe {
                arch::lapic::send_sipi(cpu.apic_id, SIPI_VECTOR);
            }
        }
    }
}

static COUNT: AtomicU32 = AtomicU32::new(1);

// Wait up to 500 ms for all APs to mark themselves up from high
// level code; they do this by calling `signal_ap` below.
fn wait_for_aps(cpus: &'static [EntryCPU]) {
    for _ in 0..(500 * 1000) {
        if COUNT.load(Ordering::Acquire) as usize == cpus.len() {
            return;
        }
        arch::cpu::pause(Duration::from_micros(1));
    }
    panic!("APs not started");
}

/// Signals that the given processor is up by incrementing
/// `COUNT`.
pub fn signal_ap(_cpu: arch::ProcessorID) {
    COUNT.fetch_add(1, Ordering::Release);
}
