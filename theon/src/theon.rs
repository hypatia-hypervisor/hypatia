// Copyright 2022  The Hypatia Authors
// All rights reserved
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use arch::HPA;

/// Returns the address of the end of the BSS segment, which
/// marks the end of the executable theon image loaded by the
/// zeroth stage loader.
pub(crate) fn end_addr() -> *const u8 {
    unsafe extern "C" {
        static end: [u8; 0];
    }
    unsafe { end.as_ptr() }
}

pub(crate) fn apstart() -> &'static [u8] {
    unsafe extern "C" {
        static apstart: [u8; 0];
        static eapstart: [u8; 0];
    }
    unsafe {
        let ptr = apstart.as_ptr();
        let len = eapstart.as_ptr().addr() - ptr.addr();
        core::slice::from_raw_parts(ptr, len)
    }
}

/// The start of Theon's virtual address space.
pub(crate) const VZERO: *const u8 = core::ptr::without_provenance(0xFFFF_8000_0000_0000);

/// Returns the raw virtual address of the given HPA relative
/// to theon's address space.
pub(crate) const fn vaddr(hpa: HPA) -> *const u8 {
    unsafe { VZERO.add(hpa.addr() as usize) }
}
