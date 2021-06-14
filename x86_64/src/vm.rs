// Copyright 2021  The Hypatia Authors
// All rights reserved
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

/// # x86_64 recursive page map implementation
///
/// Hypatia uses recursive page tables with side-loading for
/// address space inspection and manipulation.
use crate::HPA;
use bitflags::bitflags;
use core::marker::PhantomData;
use core::sync::atomic::{AtomicU64, Ordering};

bitflags! {
    pub struct EntryFlags: u64 {
        const PRESENT = 1;
        const WRITE   = 1 << 1;
        const USER    = 1 << 2;
        const WRTHRU  = 1 << 3;
        const NOCACHE = 1 << 4;
        const ACCESS  = 1 << 5;
        const DIRTY   = 1 << 6;
        const HUGE    = 1 << 7;
        const GLOBAL  = 1 << 8;
        const NX      = 1 << 63;
    }
}

///
/// Page table entries are 64-bit integers, but we must be
/// careful when accessing them, so we define them in terms
/// of atomics numbers.
///
#[repr(transparent)]
#[derive(Debug)]
pub struct Entry(AtomicU64);

impl Entry {
    const PPN_MASK: u64 = 0x0000_7FFF_FFFF_F000;

    pub fn new(hpa: HPA) -> Entry {
        Entry(AtomicU64::new(hpa.address()))
    }

    pub const fn empty() -> Entry {
        Entry(AtomicU64::new(0))
    }

    pub fn clear(&self) {
        self.0.store(0, Ordering::Relaxed)
    }

    pub fn enable(&self) {
        self.0.fetch_or(EntryFlags::PRESENT.bits, Ordering::AcqRel);
    }

    pub fn disable(&self) {
        self.0.fetch_and(!EntryFlags::PRESENT.bits, Ordering::AcqRel);
    }

    pub fn pfn(&self) -> HPA {
        HPA(self.0.load(Ordering::Relaxed) & Self::PPN_MASK)
    }

    pub fn flags(&self) -> EntryFlags {
        EntryFlags::from_bits_truncate(self.0.load(Ordering::Relaxed))
    }

    pub fn is_present(&self) -> bool {
        self.flags().contains(EntryFlags::PRESENT)
    }

    pub fn is_zero(&self) -> bool {
        self.0.load(Ordering::Relaxed) == 0
    }
}

impl Clone for Entry {
    fn clone(&self) -> Entry {
        Entry(AtomicU64::new(self.0.load(Ordering::Relaxed)))
    }
}

///
/// The nature of the recursive entry in the PML4 is that the
/// nodes in the paging radix trees are all accessable via fixed
/// locations in the virtual address space.  The constants below
/// are the beginnings of the virtual address regions for all
/// entries.
///
/// This also means that radix nodes at any given level of the
/// tree for contiguous regions of the virtual address space are
/// adajenct in the virtual mapping for the radix nodes, which
/// is a very useful property.
///

pub enum Level4 {}
pub enum Level3 {}
pub enum Level2 {}
pub enum Level1 {}

pub trait Node {
    const PML_BASE: usize;
    const SIDE_PML_BASE: usize;
    const PAGE_SHIFT: usize;

    fn index(va: usize) -> usize {
        const ADDRESS_BITS: usize = 48;
        let index_mask: usize = (1 << (ADDRESS_BITS - Self::PAGE_SHIFT)) - 1;
        (va >> Self::PAGE_SHIFT) & index_mask
    }

    fn entry(va: usize) -> &'static Entry {
        unsafe { &*(Self::PML_BASE as *const Entry).add(Self::index(va)) }
    }

    fn side_entry(va: usize) -> &'static Entry {
        unsafe { &*(Self::SIDE_PML_BASE as *const Entry).add(Self::index(va)) }
    }
}

impl Node for Level4 {
    const PML_BASE: usize = 0xFFFF_FFFF_FFFF_F000;
    const SIDE_PML_BASE: usize = 0xFFFF_FFFF_FFFF_E000;
    const PAGE_SHIFT: usize = 39;
}

impl Node for Level3 {
    const PML_BASE: usize = 0xFFFF_FFFF_FFE0_0000;
    const SIDE_PML_BASE: usize = 0xFFFF_FFFF_FFC0_0000;
    const PAGE_SHIFT: usize = 30;
}

impl Node for Level2 {
    const PML_BASE: usize = 0xFFFF_FFFF_C000_0000;
    const SIDE_PML_BASE: usize = 0xFFFF_FFFF_8000_0000;
    const PAGE_SHIFT: usize = 21;
}

impl Node for Level1 {
    const PML_BASE: usize = 0xFFFF_FF80_0000_0000;
    const SIDE_PML_BASE: usize = 0xFFFF_FF00_0000_0000;
    const PAGE_SHIFT: usize = 12;
}

pub trait Level: Node {
    type EntryType: Node;
}

impl Level for Level4 {
    type EntryType = Level3;
}

impl Level for Level3 {
    type EntryType = Level2;
}

impl Level for Level2 {
    type EntryType = Level1;
}

#[repr(C, align(4096))]
pub struct Table<L>
where
    L: Node,
{
    entries: [Entry; 512],
    level: PhantomData<L>,
}

impl<L> Table<L>
where
    L: Level,
{
    pub fn is_empty(&self) -> bool {
        self.entries.iter().all(|entry| entry.is_zero())
    }
}

pub type PageTable = Table<Level4>;

#[cfg(test)]
mod tests {
    use super::Node;

    #[test]
    fn level4_index() {
        use super::Level4;
        assert_eq!(Level4::index(0x0000_0000_0000_0000), 0);
        assert_eq!(Level4::index(0x0000_0000_0001_0000), 0);
        assert_eq!(Level4::index(0x0000_0000_0020_0000), 0);
        assert_eq!(Level4::index(0x0000_0000_4000_0000), 0);
        assert_eq!(Level4::index(0x0000_0080_0000_0000), 1);
        assert_eq!(Level4::index(0x0000_7FFF_FFFF_FFFF), 255);
        assert_eq!(Level4::index(0xFFFF_8000_0000_0000), 256);
        assert_eq!(Level4::index(0xFFFF_8000_0000_1000), 256);
        assert_eq!(Level4::index(0xFFFF_8000_0020_0000), 256);
        assert_eq!(Level4::index(0xFFFF_8000_4000_0000), 256);
        assert_eq!(Level4::index(0xFFFF_8080_0000_0000), 257);
        assert_eq!(Level4::index(Level4::PML_BASE), 511);
        assert_eq!(Level4::index(Level4::SIDE_PML_BASE), 511);
    }

    #[test]
    fn level3_index() {
        use super::Level3;
        const INDEX_BITS: usize = 18;
        const UPPER: usize = 1 << INDEX_BITS;
        const HALFWAY: usize = UPPER / 2;
        assert_eq!(Level3::index(0x0000_0000_0000_0000), 0);
        assert_eq!(Level3::index(0x0000_0000_0000_1000), 0);
        assert_eq!(Level3::index(0x0000_0000_0020_0000), 0);
        assert_eq!(Level3::index(0x0000_0000_4000_0000), 1);
        assert_eq!(Level3::index(0x0000_0080_0000_0000), 512);
        assert_eq!(Level3::index(0x0000_7FFF_FFFF_FFFF), HALFWAY - 1);
        assert_eq!(Level3::index(0xFFFF_8000_0000_0000), HALFWAY);
        assert_eq!(Level3::index(0xFFFF_8000_0000_1000), HALFWAY);
        assert_eq!(Level3::index(0xFFFF_8000_0020_0000), HALFWAY);
        assert_eq!(Level3::index(0xFFFF_8000_4000_0000), HALFWAY + 1);
        assert_eq!(Level3::index(0xFFFF_8080_0000_0000), HALFWAY + 512);
        assert_eq!(Level3::index(0xFFFF_FFFF_FFFF_F000), UPPER - 1);
        assert_eq!(Level3::index(0xFFFF_FFFF_FFFF_E000), UPPER - 1);
    }

    #[test]
    fn level2_index() {
        use super::Level2;
        const INDEX_BITS: usize = 27;
        const UPPER: usize = 1 << INDEX_BITS;
        const HALFWAY: usize = UPPER / 2;
        assert_eq!(Level2::index(0x0000_0000_0000_0000), 0);
        assert_eq!(Level2::index(0x0000_0000_0000_1000), 0);
        assert_eq!(Level2::index(0x0000_0000_0020_0000), 1);
        assert_eq!(Level2::index(0x0000_0000_4000_0000), 512);
        assert_eq!(Level2::index(0x0000_0080_0000_0000), 512 * 512);
        assert_eq!(Level2::index(0x0000_7FFF_FFFF_FFFF), HALFWAY - 1);
        assert_eq!(Level2::index(0xFFFF_8000_0000_0000), HALFWAY);
        assert_eq!(Level2::index(0xFFFF_8000_0000_1000), HALFWAY);
        assert_eq!(Level2::index(0xFFFF_8000_0020_0000), HALFWAY + 1);
        assert_eq!(Level2::index(0xFFFF_8000_4000_0000), HALFWAY + 512);
        assert_eq!(Level2::index(0xFFFF_8080_0000_0000), HALFWAY + 512 * 512);
        assert_eq!(Level2::index(0xFFFF_FFFF_FFFF_F000), UPPER - 1);
        assert_eq!(Level2::index(0xFFFF_FFFF_FFFF_E000), UPPER - 1);
    }

    #[test]
    fn level1_index() {
        use super::Level1;
        const INDEX_BITS: usize = 36;
        const UPPER: usize = 1 << INDEX_BITS;
        const HALFWAY: usize = UPPER / 2;
        assert_eq!(Level1::index(0x0000_0000_0000_0000), 0);
        assert_eq!(Level1::index(0x0000_0000_0000_1000), 1);
        assert_eq!(Level1::index(0x0000_0000_0020_0000), 512);
        assert_eq!(Level1::index(0x0000_0000_4000_0000), 512 * 512);
        assert_eq!(Level1::index(0x0000_0080_0000_0000), 512 * 512 * 512);
        assert_eq!(Level1::index(0x0000_7FFF_FFFF_FFFF), HALFWAY - 1);
        assert_eq!(Level1::index(0xFFFF_8000_0000_0000), HALFWAY);
        assert_eq!(Level1::index(0xFFFF_8000_0000_1000), HALFWAY + 1);
        assert_eq!(Level1::index(0xFFFF_8000_0020_0000), HALFWAY + 512);
        assert_eq!(Level1::index(0xFFFF_8000_4000_0000), HALFWAY + 512 * 512);
        assert_eq!(Level1::index(0xFFFF_8080_0000_0000), HALFWAY + 512 * 512 * 512);
        assert_eq!(Level1::index(0xFFFF_FFFF_FFFF_F000), UPPER - 1);
        assert_eq!(Level1::index(0xFFFF_FFFF_FFFF_E000), UPPER - 2);
    }
}
