// Copyright 2023  The Hypatia Authors
// All rights reserved
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use crate::Result;
use crate::theon;

use arch::HPA;
use core::{mem, ptr, slice};

mod madt;
mod rsdp;

/// The ACPI Table Header.
///
/// This is a common header that all ACPI tables other than the
/// RSDP share.
///
/// Ref: ACPI v6.4 sec 5.2.6
#[derive(Debug)]
#[repr(C)]
pub(crate) struct Header {
    pub signature: [u8; 4],
    pub length: [u8; 4],
    pub revision: u8,
    pub checksum: u8,
    pub oem_id: [u8; 6],
    pub oem_table_id: [u8; 8],
    pub oem_revision: [u8; 4],
    pub creator_id: [u8; 4],
    pub creator_revision: [u8; 4],
}

impl Header {
    pub fn checksum(&self, dp: *const u8) -> u8 {
        let partial = checksum(0, self.signature.as_slice());
        let partial = checksum(partial, self.length.as_slice());
        let partial = checksum(partial, slice::from_ref(&self.revision));
        let partial = checksum(partial, slice::from_ref(&self.checksum));
        let partial = checksum(partial, self.oem_id.as_slice());
        let partial = checksum(partial, self.oem_table_id.as_slice());
        let partial = checksum(partial, self.oem_revision.as_slice());
        let partial = checksum(partial, self.creator_id.as_slice());
        let mut sum = checksum(partial, self.creator_revision.as_slice());

        let datalen = self.len() - mem::size_of::<Header>();
        let dp = dp.wrapping_add(mem::size_of::<Header>());
        for k in 0..datalen {
            let b = unsafe { ptr::read(dp.wrapping_add(k)) };
            sum = checksum(sum, slice::from_ref(&b));
        }
        sum
    }

    pub fn len(&self) -> usize {
        u32::from_le_bytes(self.length) as usize
    }
}

/// The ACPI checksum function.
fn checksum(iv: u8, bs: &[u8]) -> u8 {
    bs.iter().fold(iv, |sum, &x| sum.wrapping_add(x))
}

pub(crate) fn init() -> Result<&'static [*const Header]> {
    let (acpi_region, acpi_len) = acpi_region();
    match rsdp::init(acpi_region, acpi_len) {
        Err(_) => {
            let (ebda_region, ebda_len) = ebda_region();
            rsdp::init(ebda_region, ebda_len)
        }
        rsdp => rsdp,
    }
}

pub(crate) fn parse(addrs: &[*const Header]) {
    for &addr in addrs {
        let header = unsafe { ptr::read_unaligned(addr) };
        let sig = core::str::from_utf8(&header.signature).unwrap();
        uart::panic_println!("table@{addr:x?} is {sig}");
        if sig == "APIC" {
            let cpus = madt::parse(&header, addr.cast());
            uart::panic_println!("cpus = {cpus:#x?}");
        }
    }
}

fn acpi_region() -> (*const u8, usize) {
    const ACPI_REGION_RAW: u64 = 0x000E_0000;
    const ACPI_REGION_LIMIT: u64 = 0x000F_FFFF;
    const ACPI_REGION_LEN: u64 = ACPI_REGION_LIMIT - ACPI_REGION_RAW + 1;
    const ACPI_REGION: HPA = HPA::new(ACPI_REGION_RAW);
    let acpi_region = theon::vaddr(ACPI_REGION);
    (acpi_region, ACPI_REGION_LEN as usize)
}

fn ebda_region() -> (*const u8, usize) {
    const BDA_EBDA_REAL_MODE_ADDR: HPA = HPA::new(0x040E);
    let raw = theon::vaddr(BDA_EBDA_REAL_MODE_ADDR);
    let bs = unsafe { ptr::read(raw as *const [u8; 2]) };
    let ebda_raw_paddr = u64::from(u16::from_ne_bytes(bs)) << 4;
    let ebda_ptr = theon::vaddr(HPA::new(ebda_raw_paddr));
    (ebda_ptr, 1024)
}
