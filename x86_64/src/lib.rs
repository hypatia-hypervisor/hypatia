// Copyright 2021  The Hypatia Authors
// All rights reserved
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

#![feature(asm)]
#![feature(naked_functions)]
#![cfg_attr(not(test), no_std)]

use zerocopy::FromBytes;

pub mod cpu;
pub mod io;
pub mod trap;
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

/// Useful constants for sizes.
pub const TIB: usize = 1 << 40;
pub const GIB: usize = 1 << 30;
pub const MIB: usize = 1 << 20;
pub const KIB: usize = 1 << 10;

/// Host Physical Address
///
/// A newtype representing a host physical address.
///
#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
pub struct HPA(u64);

impl HPA {
    /// Creates a new HPA from a u64.
    pub const fn new(addr: u64) -> HPA {
        HPA(addr)
    }

    /// Returns the address of this HPA as a u64.
    pub const fn address(self) -> u64 {
        self.0
    }

    pub const fn offset(self, offset: u64) -> HPA {
        HPA::new(self.0 + offset as u64)
    }
}

/// Page
pub trait Page {
    type FrameType: PageFrame;
    const SIZE: usize;
    const ALIGN: usize = Self::SIZE;
    const MASK: usize = Self::SIZE - 1;
}

#[repr(C, align(4096))]
#[derive(FromBytes)]
pub struct Page4K([u8; 4 * KIB]);
impl Page for Page4K {
    const SIZE: usize = core::mem::size_of::<Self>();
    type FrameType = PF4K;
}

#[repr(C, align(2097152))]
pub struct Page2M([u8; 2 * MIB]);
impl Page for Page2M {
    const SIZE: usize = core::mem::size_of::<Self>();
    type FrameType = PF2M;
}

/// XXX(cross): Rust does not support 1GiB alignment.
// #[repr(C, align(1073741824))]
#[allow(clippy::identity_op)]
#[repr(C)]
pub struct Page1G([u8; 1 * GIB]);
impl Page for Page1G {
    const SIZE: usize = core::mem::size_of::<Self>();
    type FrameType = PF1G;
}

pub trait PageFrame {
    type PageType: Page;
}

#[repr(transparent)]
pub struct PF1G(HPA);
impl PageFrame for PF1G {
    type PageType = Page1G;
}

#[repr(transparent)]
pub struct PF2M(HPA);
impl PageFrame for PF2M {
    type PageType = Page2M;
}

#[repr(transparent)]
pub struct PF4K(HPA);
impl PF4K {
    pub fn pfa(self) -> HPA {
        self.0
    }
}

impl PageFrame for PF4K {
    type PageType = Page4K;
}
