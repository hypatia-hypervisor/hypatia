// Copyright 2023  The Hypatia Authors
// All rights reserved
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use super::Header;
use crate::Result;

use alloc::vec::Vec;
use bitstruct::bitstruct;
use core::{mem, ptr};

bitstruct! {
    #[derive(Clone, Copy, Debug)]
    pub struct APICFlags(u32) {
        enabled: bool = 0;
        online_capable: bool = 1;
    }
}

mod ty {
    pub const LAPIC: u8 = 0;
    pub const LAPIC_LEN: usize = 8;

    pub const IOAPIC: u8 = 1;
    pub const IOAPIC_LEN: usize = 12;

    pub const X2LAPIC: u8 = 9;
    pub const X2LAPIC_LEN: usize = 16;
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct CPUInventory {
    cpus: &'static [arch::ProcessorID],
    ioapics: &'static [arch::IOAPIC],
}

pub(crate) fn parse(header: &Header, dp: *const u8) -> Result<CPUInventory> {
    if header.checksum(dp) != 0 {
        return Err("madt bad checksum");
    }
    let datalen = header.len() - mem::size_of::<Header>();
    let dp = dp.wrapping_add(mem::size_of::<Header>());

    let mut cpus = Vec::new();
    let mut ioapics = Vec::new();

    let mut k = 8;
    while k < datalen {
        if datalen - k < 2 {
            return Err("bad madt");
        }
        let p = dp.wrapping_add(k);
        let bs = unsafe { ptr::read(p.cast::<[u8; 2]>()) };
        let typ = bs[0];
        let len = usize::from(bs[1]);
        if k + len > datalen {
            return Err("corrupt madt");
        }
        match typ {
            ty::LAPIC if let Some(id) = parse_lapic(p) => cpus.push(id),
            ty::X2LAPIC if let Some(id) = parse_x2lapic(p) => cpus.push(id),
            ty::IOAPIC => ioapics.push(parse_ioapic(p)),
            _ => uart::panic_println!("ignoring {typ}"),
        }
        k += len;
    }
    Ok(CPUInventory { cpus: cpus.leak(), ioapics: ioapics.leak() })
}

fn parse_lapic(p: *const u8) -> Option<arch::ProcessorID> {
    let raw = unsafe { ptr::read(p.cast::<[u8; ty::LAPIC_LEN]>()) };
    assert_eq!(raw[0], ty::LAPIC);
    assert_eq!(raw[1], ty::LAPIC_LEN as u8);
    let id = u32::from(raw[3]);
    let flags = APICFlags(u32::from_le_bytes([raw[4], raw[5], raw[6], raw[7]]));
    ((flags.enabled() || flags.online_capable()) && id != 0xff).then_some(arch::ProcessorID(id))
}

fn parse_x2lapic(p: *const u8) -> Option<arch::ProcessorID> {
    let raw = unsafe { ptr::read(p.cast::<[u8; ty::X2LAPIC_LEN]>()) };
    assert_eq!(raw[0], ty::X2LAPIC);
    assert_eq!(raw[1], ty::X2LAPIC_LEN as u8);
    let id = u32::from_le_bytes([raw[4], raw[5], raw[6], raw[7]]);
    let flags = APICFlags(u32::from_le_bytes([raw[8], raw[9], raw[10], raw[11]]));
    ((flags.enabled() || flags.online_capable()) && id != 0xffff_ffff)
        .then_some(arch::ProcessorID(id))
}

fn parse_ioapic(p: *const u8) -> arch::IOAPIC {
    let raw = unsafe { ptr::read(p.cast::<[u8; ty::IOAPIC_LEN]>()) };
    assert_eq!(raw[0], 1);
    assert_eq!(raw[1], ty::IOAPIC_LEN as u8);
    let id = u32::from(raw[3]);
    let hpa = arch::HPA::new(u32::from_le_bytes([raw[4], raw[5], raw[6], raw[7]]).into());
    let gsib = u32::from_le_bytes([raw[8], raw[9], raw[10], raw[11]]);
    arch::IOAPIC::new(id, hpa, gsib)
}
