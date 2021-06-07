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
const PML4: usize = 0xFFFF_FFFF_FFFF_F000;
const PML3: usize = 0xFFFF_FFFF_FFE0_0000;
const PML2: usize = 0xFFFF_FFFF_C000_0000;
const PML1: usize = 0xFFFF_FF80_0000_0000;

const PPNX_MASK: usize = 0x1FF;

pub enum Level4 {}
pub enum Level3 {}
pub enum Level2 {}
pub enum Level1 {}

pub trait Node {
    fn index(va: usize) -> usize;
    fn entry(va: usize) -> &'static Entry;
}

impl Node for Level4 {
    fn index(va: usize) -> usize {
        (va >> 39) & PPNX_MASK
    }

    fn entry(va: usize) -> &'static Entry {
        let raw = PML4 + Self::index(va) * core::mem::size_of::<Entry>();
        unsafe { &*(raw as *const Entry) }
    }
}

impl Node for Level3 {
    fn index(va: usize) -> usize {
        (va >> 30) & PPNX_MASK
    }

    fn entry(va: usize) -> &'static Entry {
        let raw = PML3 + Self::index(va) * core::mem::size_of::<Entry>();
        unsafe { &*(raw as *const Entry) }
    }
}

impl Node for Level2 {
    fn index(va: usize) -> usize {
        (va >> 21) & PPNX_MASK
    }

    fn entry(va: usize) -> &'static Entry {
        let raw = PML2 + Self::index(va) * core::mem::size_of::<Entry>();
        unsafe { &*(raw as *const Entry) }
    }
}

impl Node for Level1 {
    fn index(va: usize) -> usize {
        (va >> 12) & PPNX_MASK
    }

    fn entry(va: usize) -> &'static Entry {
        let raw = PML1 + Self::index(va) * core::mem::size_of::<Entry>();
        unsafe { &*(raw as *const Entry) }
    }
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
