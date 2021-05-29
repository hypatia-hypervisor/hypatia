// Copyright 2021  The Hypatia Authors
// All rights reserved
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use crate::x86_64::memory;
use alloc::vec::Vec;
use multiboot::information::{MemoryManagement, MemoryType, Multiboot, PAddr};

const THEON_ZERO: usize = 0xffff_8000_0000_0000;

unsafe fn phys_to_slice(phys_addr: PAddr, len: usize) -> Option<&'static [u8]> {
    let p = (THEON_ZERO + phys_addr as usize) as *const u8;
    Some(core::slice::from_raw_parts(p, len))
}

fn end_addr() -> usize {
    extern "C" {
        static end: [u8; 0];
    }
    unsafe { end.as_ptr() as usize }
}

struct MM;

impl MemoryManagement for MM {
    unsafe fn paddr_to_slice(&self, phys_addr: PAddr, len: usize) -> Option<&'static [u8]> {
        phys_to_slice(phys_addr, len)
    }

    unsafe fn allocate(&mut self, _len: usize) -> Option<(PAddr, &mut [u8])> {
        None
    }

    unsafe fn deallocate(&mut self, addr: PAddr) {
        if addr != 0 {
            unimplemented!();
        }
    }
}

fn theon_region() -> memory::Region {
    let start = 0x0000_0000_0010_0000_u64;
    let phys_end = (end_addr() - THEON_ZERO) as u64;
    memory::Region {
        start,
        end: phys_end,
        typ: memory::Type::Loader,
    }
}

fn parse_memory(mb: &Multiboot) -> Option<Vec<memory::Region>> {
    Some(
        mb.memory_regions()?
            .map(|r| memory::Region {
                start: r.base_address().into(),
                end: r.base_address().wrapping_add(r.length()),
                typ: match r.memory_type() {
                    MemoryType::Available => memory::Type::RAM,
                    MemoryType::Reserved => memory::Type::Reserved,
                    MemoryType::ACPI => memory::Type::ACPI,
                    MemoryType::NVS => memory::Type::NonVolatile,
                    MemoryType::Defect => memory::Type::Defective,
                },
            })
            .collect(),
    )
}

#[derive(Debug)]
struct MultibootModule<'a> {
    pub bytes: &'a [u8],
    pub name: Option<&'a str>,
}

impl<'a> MultibootModule<'a> {
    fn region(&self) -> memory::Region {
        let phys_start = self.bytes.as_ptr() as usize - THEON_ZERO;
        let phys_end = phys_start.wrapping_add(self.bytes.len());
        memory::Region {
            start: phys_start as u64,
            end: phys_end as u64,
            typ: memory::Type::Module,
        }
    }
}

fn parse_modules<'a>(mb: &'a Multiboot) -> Option<Vec<MultibootModule<'a>>> {
    Some(
        mb.modules()?
            .map(|m| MultibootModule {
                bytes: unsafe { phys_to_slice(m.start, (m.end - m.start) as usize).unwrap() },
                name: m.string,
            })
            .collect(),
    )
}

pub fn init(mbinfo_phys: u64) {
    uart::panic_println!("mbinfo: {:08x}", mbinfo_phys);
    let mut mm = MM {};
    let mb = unsafe { Multiboot::from_ptr(mbinfo_phys as PAddr, &mut mm).unwrap() };
    uart::panic_println!("end = {:016x}", end_addr());
    if let Some(mut overlapping_regions) = parse_memory(&mb) {
        overlapping_regions.push(theon_region());
        for module in parse_modules(&mb).unwrap() {
            overlapping_regions.push(module.region());
            // if Some("bin.a") == module.name {
            //     let archive = goblin::archive::Archive::parse(module.bytes);
            //     uart::panic_println!("archive: {:#x?}", archive);
            // }
        }
        overlapping_regions.sort_by(|a, b| {
            use core::cmp::Ordering;
            let order = b.start.cmp(&a.start);
            if let Ordering::Equal = order {
                b.end.cmp(&a.end)
            } else {
                order
            }
        });
        // Split regions to ensure no overlap.  Sadly, right now this
        // means we have to copy the vector.
        let mut regions = Vec::new();
        let mut prev = overlapping_regions.pop().unwrap();
        while let Some(mut region) = overlapping_regions.pop() {
            if prev.start == region.start && prev.end < region.end {
                region.start = prev.end;
            } else if region.start < prev.end {
                regions.push(memory::Region {
                    start: prev.start,
                    end: region.start,
                    typ: prev.typ,
                });
                if region.end < prev.end {
                    regions.push(region);
                }
                prev.start = region.end;
                continue;
            }
            regions.push(prev);
            prev = region;
        }
        regions.push(prev);
        uart::panic_println!("regions: {:#x?}", regions);
    }
}
