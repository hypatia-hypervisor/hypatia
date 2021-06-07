// Copyright 2021  The Hypatia Authors
// All rights reserved
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

#![cfg_attr(not(test), no_std)]

use zerocopy::FromBytes;

pub mod cpu;
pub mod io;
pub mod vm;

///
/// x86_64 defines three page sizes:
///
/// * 4KiB
/// * 2MiB
/// * 1GiB
///
/// Everything in the host virtual address space
/// in Hypatia is mapped using 4KiB pages.

pub const PAGE_SIZE: usize = 4096;

#[repr(C, align(4096))]
#[derive(FromBytes)]
pub struct Page([u8; PAGE_SIZE]);

/// Host Physical Address
///
/// A newtype representing a host physical address.
///
#[repr(transparent)]
pub struct HPA(u64);

impl HPA {
    /// Creates a new HPA from a u64.
    pub fn new(addr: u64) -> HPA {
        HPA(addr)
    }

    /// Returns the address of this HPA as a u64.
    pub fn address(self) -> u64 {
        self.0
    }
}
