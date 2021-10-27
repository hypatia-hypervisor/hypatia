// Copyright 2021  The Hypatia Authors
// All rights reserved
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! # x86_64 - Interfaces to the hardware.
//!
//! The x86_64 crate provides convenience and utility code
//! for interacting with the hardware, particularly virtual
//! memory.
//!
//! # Paging related
//!
//! x86_64 defines three page sizes:
//!
//! * 4KiB
//! * 2MiB
//! * 1GiB
//!
//! Everything in the host virtual address space
//! in Hypatia is mapped using 4KiB pages, but we
//! provide types that correspond to each size.
//!
//! In general, for each size we provide three
//! types, each made generic by a trait:
//!
//! Page
//! : A page wraps the contents of a page of virtual
//! memory.  That is, it actually contains bytes.  The
//! page must be mapped for a variable of this type to
//! have meaning.
//!
//! PageFrame
//! : A PageFrame is the physical memory that a page
//! refers to, in the physical address space.  It wraps
//! an HPA.  Since it is in the physical, not virtual,
//! address space, it cannot be mapped.
//!
//! VPageAddr
//! : A VPageAddr is the aligned virtual address of a
//! page in some address space.  The page may or may
//! not be mapped.

#![feature(asm)]
#![feature(naked_functions)]
#![feature(step_trait)]
#![cfg_attr(not(test), no_std)]

use core::iter::Step;
use zerocopy::FromBytes;

pub mod cpu;
pub mod io;
pub mod trap;
pub mod vm;

/// Useful constants for sizes.
pub const TIB: usize = 1 << 40;
pub const GIB: usize = 1 << 30;
pub const MIB: usize = 1 << 20;
pub const KIB: usize = 1 << 10;

/// Host Physical Address
///
/// A newtype representing a host physical address.
/// Note that this address is in the physical address
/// space and is not mapped (it may inadvertently be
/// identity mapped into any given virtual address space,
/// but purely by happenstance and in general that is
/// not true).
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

/// Page represents a page of some size that is mapped into
/// the virtual address space.
pub trait Page {
    type FrameType: PageFrame;
    type VPageAddrType: VPageAddr;
    const SIZE: usize;
    const ALIGN: usize = Self::SIZE;
    const MASK: usize = Self::SIZE - 1;

    fn vaddr(&self) -> Self::VPageAddrType;
}

#[repr(C, align(4096))]
#[derive(FromBytes)]
pub struct Page4K([u8; 4 * KIB]);
impl Page for Page4K {
    const SIZE: usize = core::mem::size_of::<Self>();
    type FrameType = PF4K;
    type VPageAddrType = V4KA;

    fn vaddr(&self) -> V4KA {
        V4KA(self.0.as_ptr() as usize)
    }
}

#[repr(C, align(2097152))]
pub struct Page2M([u8; 2 * MIB]);
impl Page for Page2M {
    const SIZE: usize = core::mem::size_of::<Self>();
    type FrameType = PF2M;
    type VPageAddrType = V2MA;

    fn vaddr(&self) -> V2MA {
        V2MA(self.0.as_ptr() as usize)
    }
}

/// XXX(cross): Rust does not support 1GiB alignment.
// #[repr(C, align(1073741824))]
#[allow(clippy::identity_op)]
#[repr(C)]
pub struct Page1G([u8; 1 * GIB]);
impl Page for Page1G {
    const SIZE: usize = core::mem::size_of::<Self>();
    type FrameType = PF1G;
    type VPageAddrType = V1GA;

    fn vaddr(&self) -> V1GA {
        V1GA(self.0.as_ptr() as usize)
    }
}

/// XXX(cross): Rust really does not support 512GiB alignment.
// #[repr(C, align(549755813888))]
#[repr(C)]
pub struct Page512G([u8; 512 * GIB]);
impl Page for Page512G {
    const SIZE: usize = core::mem::size_of::<Self>();
    type FrameType = PF512G;
    type VPageAddrType = V512GA;

    fn vaddr(&self) -> V512GA {
        V512GA(self.0.as_ptr() as usize)
    }
}

pub trait PageFrame {
    type PageType: Page;
}

#[repr(transparent)]
pub struct PF512G(HPA);
impl PageFrame for PF512G {
    type PageType = Page512G;
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

/// Types implementing the VPageAddr trait represent page-aligned
/// virtual address, for varying page sizes.
///
/// XXX(cross): It would be nice to generalize this somehow
/// so that they weren't so specific to pages.
pub trait VPageAddr: Sized {
    type PageType: Page;

    fn new(va: usize) -> Self;

    fn new_round_down(va: usize) -> Self {
        Self::new(va & !Self::PageType::MASK)
    }

    fn new_round_up(va: usize) -> Self {
        Self::new(va.wrapping_add(Self::PageType::MASK) & !Self::PageType::MASK)
    }

    fn address(self) -> usize;
}

/// A type representing a 4KiB-aligned page address.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd)]
pub struct V4KA(usize);

impl VPageAddr for V4KA {
    type PageType = Page4K;

    fn new(va: usize) -> V4KA {
        assert_eq!(va & Page4K::MASK, 0);
        V4KA(va)
    }

    fn address(self) -> usize {
        self.0
    }
}

impl Step for V4KA {
    fn steps_between(start: &V4KA, end: &V4KA) -> Option<usize> {
        let diff = end.0.checked_sub(start.0)?;
        Some(diff / Page4K::SIZE)
    }

    fn forward_checked(start: Self, count: usize) -> Option<Self> {
        let diff = count.checked_mul(Page4K::SIZE)?;
        let fwd = start.0.checked_add(diff)?;
        Some(V4KA(fwd))
    }

    fn backward_checked(start: Self, count: usize) -> Option<Self> {
        let diff = count.checked_mul(Page4K::SIZE)?;
        let bck = start.0.checked_sub(diff)?;
        Some(V4KA(bck))
    }
}

#[cfg(test)]
mod v4ka_tests {
    use super::*;

    #[test]
    fn steps_between_works() {
        let start = V4KA::new(0);
        let end = V4KA::new(0);
        assert_eq!(V4KA::steps_between(&start, &end), Some(0));

        let end = V4KA::new_round_up(1);
        assert_eq!(end.address(), 4096);
        assert_eq!(V4KA::steps_between(&start, &end), Some(1));

        let end = V4KA::new(16 * KIB);
        assert_eq!(V4KA::steps_between(&start, &end), Some(4));

        assert_eq!(V4KA::steps_between(&end, &start), None);
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd)]
pub struct V2MA(usize);

impl VPageAddr for V2MA {
    type PageType = Page2M;

    fn new(va: usize) -> V2MA {
        assert_eq!(va & Page2M::MASK, 0);
        V2MA(va)
    }

    fn address(self) -> usize {
        self.0
    }
}

impl Step for V2MA {
    fn steps_between(start: &V2MA, end: &V2MA) -> Option<usize> {
        let diff = end.0.checked_sub(start.0)?;
        Some(diff / Page2M::SIZE)
    }

    fn forward_checked(start: Self, count: usize) -> Option<Self> {
        let diff = count.checked_mul(Page2M::SIZE)?;
        let fwd = start.0.checked_add(diff)?;
        Some(V2MA(fwd))
    }

    fn backward_checked(start: Self, count: usize) -> Option<Self> {
        let diff = count.checked_mul(Page2M::SIZE)?;
        let bck = start.0.checked_sub(diff)?;
        Some(V2MA(bck))
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd)]
pub struct V1GA(usize);

impl VPageAddr for V1GA {
    type PageType = Page1G;

    fn new(va: usize) -> V1GA {
        assert_eq!(va & Page1G::MASK, 0);
        V1GA(va)
    }

    fn address(self) -> usize {
        self.0
    }
}

impl Step for V1GA {
    fn steps_between(start: &V1GA, end: &V1GA) -> Option<usize> {
        let diff = end.0.checked_sub(start.0)?;
        Some(diff / Page1G::SIZE)
    }

    fn forward_checked(start: Self, count: usize) -> Option<Self> {
        let diff = count.checked_mul(Page1G::SIZE)?;
        let fwd = start.0.checked_add(diff)?;
        Some(V1GA(fwd))
    }

    fn backward_checked(start: Self, count: usize) -> Option<Self> {
        let diff = count.checked_mul(Page1G::SIZE)?;
        let bck = start.0.checked_sub(diff)?;
        Some(V1GA(bck))
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd)]
pub struct V512GA(usize);

impl VPageAddr for V512GA {
    type PageType = Page512G;

    fn new(va: usize) -> V512GA {
        assert_eq!(va & Page512G::MASK, 0);
        V512GA(va)
    }

    fn address(self) -> usize {
        self.0
    }
}

impl Step for V512GA {
    fn steps_between(start: &V512GA, end: &V512GA) -> Option<usize> {
        let diff = end.0.checked_sub(start.0)?;
        Some(diff / Page512G::SIZE)
    }

    fn forward_checked(start: Self, count: usize) -> Option<Self> {
        let diff = count.checked_mul(Page512G::SIZE)?;
        let fwd = start.0.checked_add(diff)?;
        Some(V512GA(fwd))
    }

    fn backward_checked(start: Self, count: usize) -> Option<Self> {
        let diff = count.checked_mul(Page512G::SIZE)?;
        let bck = start.0.checked_sub(diff)?;
        Some(V512GA(bck))
    }
}
