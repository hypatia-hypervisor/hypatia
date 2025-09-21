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

#![feature(assert_matches)]
#![feature(fn_align)]
#![feature(step_trait)]
#![cfg_attr(not(test), no_std)]
#![forbid(absolute_paths_not_starting_with_crate)]
#![forbid(elided_lifetimes_in_paths)]
#![forbid(unsafe_op_in_unsafe_fn)]

use core::convert::TryFrom;
use core::fmt::Debug;
use core::iter::Step;
use zerocopy::FromBytes;

pub mod cpu;
pub(crate) mod debug;
pub mod gdt;
pub mod idt;
pub mod io;
pub mod lapic;
pub mod segment;
pub mod trap;
pub mod tss;
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
    pub const fn addr(self) -> u64 {
        self.0
    }

    #[must_use]
    pub const fn offset(self, offset: usize) -> HPA {
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

    fn frame(&self) -> Self::FrameType {
        let addr = self.vaddr().addr();
        let pfa = vm::translate(addr).expect("Page is mapped");
        Self::FrameType::new(pfa)
    }
}

#[derive(FromBytes)]
#[repr(C, align(4096))]
pub struct Page4K([u8; 4 * KIB]);

impl Page4K {
    /// Returns a new, zeroed page.
    pub const fn new() -> Page4K {
        Page4K([0; 4 * KIB])
    }

    /// Returns an invalid pointer.
    pub const fn proto_ptr() -> *const Page4K {
        core::ptr::without_provenance(0)
    }
}

impl Default for Page4K {
    fn default() -> Self {
        Self::new()
    }
}

impl Page for Page4K {
    const SIZE: usize = core::mem::size_of::<Self>();
    type FrameType = PF4K;
    type VPageAddrType = V4KA;

    fn vaddr(&self) -> V4KA {
        V4KA(self.0.as_ptr().addr())
    }
}

#[repr(C, align(2097152))]
pub struct Page2M([u8; 2 * MIB]);
impl Page for Page2M {
    const SIZE: usize = core::mem::size_of::<Self>();
    type FrameType = PF2M;
    type VPageAddrType = V2MA;

    fn vaddr(&self) -> V2MA {
        V2MA(self.0.as_ptr().addr())
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
        V1GA(self.0.as_ptr().addr())
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
        V512GA(self.0.as_ptr().addr())
    }
}

pub trait PageFrame {
    type PageType: Page;
    fn new(pfa: HPA) -> Self;
}

#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
pub struct PF512G(HPA);
impl PageFrame for PF512G {
    type PageType = Page512G;
    fn new(pfa: HPA) -> Self {
        Self(pfa)
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
pub struct PF1G(HPA);
impl PageFrame for PF1G {
    type PageType = Page1G;
    fn new(pfa: HPA) -> Self {
        Self(pfa)
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
pub struct PF2M(HPA);
impl PageFrame for PF2M {
    type PageType = Page2M;
    fn new(pfa: HPA) -> Self {
        Self(pfa)
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
pub struct PF4K(HPA);
impl PF4K {
    pub fn pfa(self) -> HPA {
        self.0
    }
}

impl PageFrame for PF4K {
    type PageType = Page4K;
    fn new(pfa: HPA) -> Self {
        Self(pfa)
    }
}

/// Types implementing the VPageAddr trait represent page-aligned
/// virtual address, for varying page sizes.
///
/// XXX(cross): It would be nice to generalize this somehow
/// so that they weren't so specific to pages.
pub trait VPageAddr: Sized + Debug + Clone + Copy {
    type PageType: Page;

    fn new(va: usize) -> Self;

    fn new_round_down(va: usize) -> Self {
        Self::new(va & !Self::PageType::MASK)
    }

    fn new_round_up(va: usize) -> Self {
        Self::new(va.wrapping_add(Self::PageType::MASK) & !Self::PageType::MASK)
    }

    fn addr(self) -> usize;
}

/// A type representing a 4KiB-aligned page address.
#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct V4KA(usize);

impl VPageAddr for V4KA {
    type PageType = Page4K;

    fn new(va: usize) -> V4KA {
        assert_eq!(va & Page4K::MASK, 0);
        V4KA(va)
    }

    fn addr(self) -> usize {
        self.0
    }
}

impl Step for V4KA {
    fn steps_between(start: &V4KA, end: &V4KA) -> (usize, Option<usize>) {
        let start = start.0 / Page4K::SIZE;
        let end = end.0 / Page4K::SIZE;
        usize::steps_between(&start, &end)
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
        assert_eq!(V4KA::steps_between(&start, &end), (0, Some(0)));

        let end = V4KA::new_round_up(1);
        assert_eq!(end.addr(), 4096);
        assert_eq!(V4KA::steps_between(&start, &end), (1, Some(1)));

        let end = V4KA::new(16 * KIB);
        assert_eq!(V4KA::steps_between(&start, &end), (4, Some(4)));

        assert_eq!(V4KA::steps_between(&end, &start), (0, None));
    }
}

/// A type representation a 2MiB aligned address.
#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct V2MA(usize);

impl VPageAddr for V2MA {
    type PageType = Page2M;

    fn new(va: usize) -> V2MA {
        assert_eq!(va & Page2M::MASK, 0);
        V2MA(va)
    }

    fn addr(self) -> usize {
        self.0
    }
}

impl Step for V2MA {
    fn steps_between(start: &V2MA, end: &V2MA) -> (usize, Option<usize>) {
        let start = start.0 / Page2M::SIZE;
        let end = end.0 / Page2M::SIZE;
        usize::steps_between(&start, &end)
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

/// A type representing a 1GiB aligned address.
#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct V1GA(usize);

impl VPageAddr for V1GA {
    type PageType = Page1G;

    fn new(va: usize) -> V1GA {
        assert_eq!(va & Page1G::MASK, 0);
        V1GA(va)
    }

    fn addr(self) -> usize {
        self.0
    }
}

impl Step for V1GA {
    fn steps_between(start: &V1GA, end: &V1GA) -> (usize, Option<usize>) {
        let start = start.0 / Page1G::SIZE;
        let end = end.0 / Page1G::SIZE;
        usize::steps_between(&start, &end)
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

/// A type representing a 512GiB aligned address.
#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct V512GA(usize);

impl VPageAddr for V512GA {
    type PageType = Page512G;

    fn new(va: usize) -> V512GA {
        assert_eq!(va & Page512G::MASK, 0);
        V512GA(va)
    }

    fn addr(self) -> usize {
        self.0
    }
}

impl Step for V512GA {
    fn steps_between(start: &V512GA, end: &V512GA) -> (usize, Option<usize>) {
        let start = start.0 / Page512G::SIZE;
        let end = end.0 / Page512G::SIZE;
        usize::steps_between(&start, &end)
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StackIndex {
    Rsp0 = 0,
    Ist1 = 1,
    Ist2 = 2,
    Ist3 = 3,
    Ist4 = 4,
    Ist5 = 5,
    Ist6 = 6,
    Ist7 = 7,
}

pub struct HyperStack {
    address: *const u8,
    size: usize,
}

impl HyperStack {
    pub fn top(&self) -> *const u8 {
        unsafe { self.address.add(self.size) }
    }
}

/// CPU Protection Levels
///
/// On x86_64, lower (Ring0) is more privileged than higher.
/// On 64-bit, we really only use Ring0 ("kernel" mode) and
/// Ring3 ("user" mode).  In Hypatia, we actually only use
/// kernel mode.
pub enum CPL {
    Ring0,
    Ring1,
    Ring2,
    Ring3,
}

impl TryFrom<u8> for CPL {
    type Error = &'static str;
    fn try_from(raw: u8) -> Result<Self, Self::Error> {
        match raw {
            0b00 => Ok(CPL::Ring0),
            0b01 => Ok(CPL::Ring1),
            0b10 => Ok(CPL::Ring2),
            0b11 => Ok(CPL::Ring3),
            _ => Err("unrepresentable value in raw privilege level"),
        }
    }
}

/// A Processor ID.  We'd call this CPUID, but that
/// conflicts with the similarly named instruction.
///
/// Note that this is repr transparent and exactly
/// 32 bits wide; this is important as values of
/// this type are accessed from assembly language
/// during AP startup.
#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
pub struct ProcessorID(pub u32);

impl From<ProcessorID> for u32 {
    fn from(id: ProcessorID) -> u32 {
        id.0
    }
}
