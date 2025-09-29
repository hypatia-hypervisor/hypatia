// Copyright 2023  The Hypatia Authors
// All rights reserved
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! The Root System Description Pointer.
//!
//! This table specifies the ACPI version and points to the
//! system descriptor table.  Unlike other ACPI tables, the
//! format of this table is unique and it does not share the
//! common table header.  Moreover, it is dependent on the
//! ACPI version, which is contained in the table itself.
//! Thus, we do not define a structure for it, but rather
//! treat it specially, reading parts and dissecting them
//! by hand.
//!
//! A notional struct definition might be:
//!
//! pub(crate) struct RSDP {
//!     pub signature: [u8; 8],
//!     pub checksum: u8,
//!     pub oem_id: [u8; 6],
//!     pub revision: u8,
//!     pub rsdt_addr: [u8; 4],
//!     // Specific to the ACPI >1.0 RSDP
//!     pub length: [u8; 4],
//!     pub xsdt_addr: [u8; 8],
//!     pub extended_cksum: u8,
//!     reserved: [u8; 3],
//! }
//!
//! Ref: ACPI v6.4 sec 5.2.5.3

use super::{Header, checksum};
use crate::Result;
use crate::Vec;
use crate::theon;

use core::mem;
use core::ptr;

use arch::HPA;

fn is_version1(bs: &[u8; 20]) -> bool {
    const ACPI_REVISION_INDEX: usize = 15;
    bs[ACPI_REVISION_INDEX] == 0
}

/// Find the RSDP in some bounded region.
/// Returns a Result over the associated SDT.
pub(super) fn init(mut va: *const u8, len: usize) -> Result<&'static [*const Header]> {
    const XSDP_RAW_LEN: usize = 36;
    const RSDP_RAW_LEN: usize = 20;

    let end = va.wrapping_add(len);
    if !va.is_aligned_to(2) || !end.is_aligned_to(2) {
        return Err("region misaligned");
    }

    while va != end {
        if end.addr() - va.addr() < RSDP_RAW_LEN {
            return Err("region too small");
        }
        let raw = unsafe { ptr::read(va as *const [u8; RSDP_RAW_LEN]) };
        if raw[0..8] != *b"RSD PTR " {
            va = va.wrapping_add(2);
            continue;
        }
        if checksum(0, &raw) != 0 {
            return Err("bad RSDPv1 checksum");
        }
        let is_v1 = is_version1(&raw);
        let sdt_phys_addr = if is_v1 {
            let addr = u32::from_ne_bytes(raw[16..20].try_into().unwrap());
            u64::from(addr)
        } else {
            if end.addr() - va.addr() < XSDP_RAW_LEN {
                return Err("region too small");
            }
            let raw = unsafe { ptr::read(va as *const [u8; XSDP_RAW_LEN]) };
            let len = u32::from_ne_bytes([raw[20], raw[21], raw[22], raw[23]]);
            if len as usize != XSDP_RAW_LEN {
                return Err("RSDP wrong length");
            }
            if checksum(0, &raw) != 0 {
                return Err("bad RSDPv2 checksum");
            }
            u64::from_ne_bytes(raw[24..32].try_into().unwrap())
        };
        let sdt_ptr = theon::vaddr(HPA::new(sdt_phys_addr)).cast::<Header>();
        let header = unsafe { ptr::read_unaligned(sdt_ptr) };
        let data_ptr = sdt_ptr.wrapping_add(1);
        let dlen = u32::from_ne_bytes(header.length) as usize - mem::size_of::<Header>();
        let len = dlen / if is_v1 { mem::size_of::<u32>() } else { mem::size_of::<u64>() };
        let mut addrs = Vec::with_capacity(len);
        for k in 0..len {
            let addr = if is_v1 {
                let ptr = data_ptr.cast::<u32>().wrapping_add(k);
                u64::from(unsafe { ptr::read_unaligned(ptr) })
            } else {
                let ptr = data_ptr.cast::<u64>().wrapping_add(k);
                unsafe { ptr::read_unaligned(ptr) }
            };
            let hpa = HPA::new(addr);
            let vaddr = theon::vaddr(hpa).cast::<Header>();
            addrs.push(vaddr);
        }
        return Ok(addrs.leak());
    }
    Err("Could not find an RSDP")
}
