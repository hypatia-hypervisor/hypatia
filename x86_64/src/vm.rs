// Copyright 2021  The Hypatia Authors
// All rights reserved
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! # x86_64 recursive page map implementation
//!
//! Hypatia uses recursive page tables with side-loading for
//! address space inspection and manipulation.

use crate::{HPA, PF1G, PF2M, PF4K, Page, PageFrame, V1GA, V2MA, V4KA, V512GA, VPageAddr};
use bitflags::bitflags;
use core::ops::Range;
//use core::marker::PhantomData;    // XXX(cross): Not yet.
use core::sync::atomic::{AtomicU64, Ordering};

pub type Result<T> = core::result::Result<T, &'static str>;

bitflags! {
    #[derive(Clone, Copy, Debug)]
    pub struct PTEFlags: u64 {
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

/// Page table entries are 64-bit integers, but we must be
/// careful when accessing them, so we define them in terms
/// of atomics.
#[repr(transparent)]
pub struct PTE(AtomicU64);

impl PTE {
    const PFA_MASK: u64 = 0x0000_7FFF_FFFF_F000;

    /// Creates a new PTE from the given HPA and flags.
    ///
    /// TODO(cross): Extend this to be generic over the
    /// physical frame types defined in lib.rs.
    pub fn new(hpa: HPA, flags: PTEFlags) -> PTE {
        let address = hpa.addr() & Self::PFA_MASK;
        assert_eq!(hpa.addr(), address);
        PTE(AtomicU64::new(address | flags.bits()))
    }

    /// Creates an empty (zero) PTE.
    pub const fn empty() -> PTE {
        PTE(AtomicU64::new(0))
    }

    /// Creates an invalid prototype pointer for provenance casts.
    pub const fn proto_ptr() -> *const PTE {
        core::ptr::null()
    }

    /// Zeroes out the PTE.
    pub fn clear(&self) {
        self.0.store(0, Ordering::Relaxed)
    }

    /// Sets the "PRESENT" bit in the PTE.
    pub fn enable(&self) {
        self.0.fetch_or(PTEFlags::PRESENT.bits(), Ordering::AcqRel);
    }

    /// Clears the present bit in the PTE, disabling access to the region
    /// it describes.
    pub fn disable(&self) {
        self.0.fetch_and(!PTEFlags::PRESENT.bits(), Ordering::AcqRel);
    }

    /// Assign self the value of the given PTE.
    pub fn assign(&self, pte: PTE) {
        self.0.store(pte.0.into_inner(), Ordering::Relaxed);
    }

    /// Returns the physical frame address associated with the PTE.
    pub fn pfa(&self) -> HPA {
        HPA(self.0.load(Ordering::Relaxed) & Self::PFA_MASK)
    }

    /// Extracts and returns the flags attached to this PTE.
    pub fn flags(&self) -> PTEFlags {
        PTEFlags::from_bits_truncate(self.0.load(Ordering::Relaxed))
    }

    /// Returns true iff the PTE is marked "PRESENT".
    pub fn is_present(&self) -> bool {
        self.flags().contains(PTEFlags::PRESENT)
    }

    /// Returns true iff the bit marking this either a huge or large page is set.
    pub fn is_big(&self) -> bool {
        self.flags().contains(PTEFlags::HUGE)
    }

    /// Returns true iff the entry is zero.
    pub fn is_zero(&self) -> bool {
        self.0.load(Ordering::Relaxed) == 0
    }
}

impl Clone for PTE {
    fn clone(&self) -> PTE {
        PTE(AtomicU64::new(self.0.load(Ordering::Relaxed)))
    }
}

impl core::fmt::Debug for PTE {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let flags = self.flags();
        let flag_or = |f: PTEFlags, a, b| {
            if flags.contains(f) { a } else { b }
        };
        f.write_str(flag_or(PTEFlags::NX, "-", "X"))?;
        f.write_fmt(format_args!(":{:#x?}:", self.pfa().addr()))?;
        f.write_str(flag_or(PTEFlags::GLOBAL, "G", "-"))?;
        f.write_str(flag_or(PTEFlags::HUGE, "H", "-"))?;
        f.write_str(flag_or(PTEFlags::DIRTY, "D", "-"))?;
        f.write_str(flag_or(PTEFlags::ACCESS, "A", "-"))?;
        f.write_str(flag_or(PTEFlags::NOCACHE, "C̶", "-"))?;
        f.write_str(flag_or(PTEFlags::USER, "U", "-"))?;
        f.write_str(flag_or(PTEFlags::WRITE, "W", "-"))?;
        f.write_str(flag_or(PTEFlags::PRESENT, "R", "-"))
    }
}

// XXX: Figure out why Rust thinks this Entry is unused.
#[allow(dead_code)]
trait Entry {}

#[repr(u64)]
enum L4E {
    #[allow(dead_code)]
    Next(PTE),
}
impl Entry for L4E {}

#[repr(u64)]
enum L3E {
    #[allow(dead_code)]
    Next(PTE),
    Page(PF1G),
}
impl Entry for L3E {}

#[repr(u64)]
enum L2E {
    #[allow(dead_code)]
    Next(PTE),
    Page(PF2M),
}
impl Entry for L2E {}

enum L1E {
    Page(PF4K),
}
impl Entry for L1E {}

///
/// The nature of the recursive entry in the table root is that
/// the nodes in the paging radix trees are all accessible via
/// fixed locations in the virtual address space.  The constants
/// below are the beginnings of the virtual address regions for
/// all entries.
///
/// This also means that radix nodes at any given level of the
/// tree for contiguous regions of the virtual address space are
/// adajenct in the virtual mapping for the radix nodes, which
/// is a very useful property.
///
trait Level {
    type EntryType: Entry;
    type VPageAddrType: VPageAddr + core::iter::Step;
    const BASE_ADDRESS: usize;
    const SIDE_BASE_ADDRESS: usize;
    const PAGE_SHIFT: usize;

    fn index(va: usize) -> usize {
        const WORD_SIZE: usize = 64;
        const ADDRESS_BITS: usize = 48;
        const SIGN_EXTENSION_BITS: usize = WORD_SIZE - ADDRESS_BITS;
        const ADDRESS_MASK: usize = !0 >> SIGN_EXTENSION_BITS;
        (va & ADDRESS_MASK) >> Self::PAGE_SHIFT
    }

    fn decode(pte: PTE) -> Option<Self::EntryType>;

    fn pte_ref(va: usize) -> &'static PTE {
        let addr = Self::BASE_ADDRESS + Self::index(va) * core::mem::size_of::<PTE>();
        unsafe { &*PTE::proto_ptr().with_addr(addr) }
    }

    fn entry(va: usize) -> Option<Self::EntryType> {
        let pte = Self::pte_ref(va).clone();
        Self::decode(pte)
    }

    fn set_entry(va: usize, pte: PTE) {
        let entry = Self::pte_ref(va);
        entry.assign(pte);
    }

    fn clear(va: usize) {
        let entry = Self::pte_ref(va);
        entry.clear();
    }

    /// # Safety
    ///
    /// This is not safe.  It requires that some address space is side-loaded
    /// before calling.
    unsafe fn side_pte_ref(va: usize) -> &'static PTE {
        let addr = Self::SIDE_BASE_ADDRESS + Self::index(va) * core::mem::size_of::<PTE>();
        unsafe { &*PTE::proto_ptr().with_addr(addr) }
    }

    /// # Safety
    ///
    /// This is not safe.  It requires that some address space is side-loaded
    /// before calling.
    unsafe fn side_entry(va: usize) -> Option<Self::EntryType> {
        let pte = unsafe { Self::side_pte_ref(va).clone() };
        Self::decode(pte)
    }

    /// # Safety
    ///
    /// This is not safe.  It requires that some address space is side-loaded
    /// before calling.
    unsafe fn set_side_entry(va: usize, pte: PTE) {
        let entry = unsafe { Self::side_pte_ref(va) };
        entry.assign(pte);
    }

    /// # Safety
    ///
    /// This is note safe.  It rquires that some address space is side-loaded
    /// before calling.
    unsafe fn make_side_level<A>(va: V4KA, allocator: &mut A) -> Result<()>
    where
        A: FnMut() -> Result<PF4K>;
}

enum Level4 {}
enum Level3 {}
enum Level2 {}
enum Level1 {}

impl Level4 {
    #[cfg(test)]
    const SELF_INDEX: usize = 511;
    const SIDE_INDEX: usize = 510;
}

impl Level for Level4 {
    type EntryType = L4E;
    type VPageAddrType = V512GA;
    const BASE_ADDRESS: usize = 0xFFFF_FFFF_FFFF_F000;
    const SIDE_BASE_ADDRESS: usize = 0xFFFF_FFFF_FFFF_E000;
    const PAGE_SHIFT: usize = 39;

    fn decode(pte: PTE) -> Option<Self::EntryType> {
        if pte.is_present() { Some(L4E::Next(pte)) } else { None }
    }

    unsafe fn make_side_level<A>(va: V4KA, allocator: &mut A) -> Result<()>
    where
        A: FnMut() -> Result<PF4K>,
    {
        unsafe {
            if Level4::side_entry(va.addr()).is_none() {
                Level4::set_side_entry(va.addr(), alloc_inner(allocator)?);
            }
        }
        Ok(())
    }
}

impl Level for Level3 {
    type EntryType = L3E;
    type VPageAddrType = V1GA;
    const BASE_ADDRESS: usize = 0xFFFF_FFFF_FFE0_0000;
    const SIDE_BASE_ADDRESS: usize = 0xFFFF_FFFF_FFC0_0000;
    const PAGE_SHIFT: usize = 30;

    fn decode(pte: PTE) -> Option<Self::EntryType> {
        if !pte.is_present() {
            None
        } else if pte.is_big() {
            Some(L3E::Page(PF1G(pte.pfa())))
        } else {
            Some(L3E::Next(pte))
        }
    }

    unsafe fn make_side_level<A>(va: V4KA, allocator: &mut A) -> Result<()>
    where
        A: FnMut() -> Result<PF4K>,
    {
        unsafe {
            Level4::make_side_level(va, allocator)?;
            if Level3::side_entry(va.addr()).is_none() {
                Level3::set_side_entry(va.addr(), alloc_inner(allocator)?);
            }
        }
        Ok(())
    }
}

impl Level for Level2 {
    type EntryType = L2E;
    type VPageAddrType = V2MA;
    const BASE_ADDRESS: usize = 0xFFFF_FFFF_C000_0000;
    const SIDE_BASE_ADDRESS: usize = 0xFFFF_FFFF_8000_0000;
    const PAGE_SHIFT: usize = 21;

    fn decode(pte: PTE) -> Option<Self::EntryType> {
        if !pte.is_present() {
            None
        } else if pte.is_big() {
            Some(L2E::Page(PF2M(pte.pfa())))
        } else {
            Some(L2E::Next(pte))
        }
    }

    unsafe fn make_side_level<A>(va: V4KA, allocator: &mut A) -> Result<()>
    where
        A: FnMut() -> Result<PF4K>,
    {
        unsafe {
            Level3::make_side_level(va, allocator)?;
            if Level2::side_entry(va.addr()).is_none() {
                Level2::set_side_entry(va.addr(), alloc_inner(allocator)?);
            }
        }
        Ok(())
    }
}

impl Level for Level1 {
    type EntryType = L1E;
    type VPageAddrType = V4KA;
    const BASE_ADDRESS: usize = 0xFFFF_FF80_0000_0000;
    const SIDE_BASE_ADDRESS: usize = 0xFFFF_FF00_0000_0000;
    const PAGE_SHIFT: usize = 12;

    fn decode(pte: PTE) -> Option<Self::EntryType> {
        if !pte.is_present() { None } else { Some(L1E::Page(PF4K(pte.pfa()))) }
    }

    unsafe fn make_side_level<A>(va: V4KA, allocator: &mut A) -> Result<()>
    where
        A: FnMut() -> Result<PF4K>,
    {
        unsafe {
            Level2::make_side_level(va, allocator)?;
            if Level1::side_entry(va.addr()).is_none() {
                Level1::set_side_entry(va.addr(), alloc_inner(allocator)?);
            }
        }
        Ok(())
    }
}

#[repr(C, align(4096))]
struct PageTable {
    entries: [PTE; 512],
}

impl PageTable {
    pub fn _is_empty(&self) -> bool {
        self.entries.iter().all(|entry| entry.is_zero())
    }

    pub fn root_addr(&self) -> HPA {
        translate_ptr(self).expect("mapped object is mapped")
    }

    pub const fn proto_ptr() -> *const PageTable {
        core::ptr::null()
    }
}

/// A walk represents a path of page table entries from the root
/// down to the leaf level of paging radix tree.
struct Walk(Option<L4E>, Option<L3E>, Option<L2E>, Option<L1E>);

/// Performs a page table walk for the virtual address of the given
/// pointer in the current address space.
#[allow(dead_code)]
fn walk_ptr<T>(p: *const T) -> Walk {
    walk(p.addr())
}

fn walk(va: usize) -> Walk {
    let pt4e = Level4::entry(va);
    match pt4e {
        Some(L4E::Next(_)) => {}
        _ => return Walk(pt4e, None, None, None),
    }

    let pt3e = Level3::entry(va);
    match pt3e {
        Some(L3E::Next(_)) => {}
        _ => return Walk(pt4e, pt3e, None, None),
    }

    let pt2e = Level2::entry(va);
    match pt2e {
        Some(L2E::Next(_)) => {}
        _ => return Walk(pt4e, pt3e, pt2e, None),
    }

    let pt1e = Level1::entry(va);

    Walk(pt4e, pt3e, pt2e, pt1e)
}

/// Translates the virtual address of the given pointer in the current
/// address space to a host physical address.
pub fn translate_ptr<T>(p: *const T) -> Option<HPA> {
    translate(p.addr())
}

pub fn translate(va: usize) -> Option<HPA> {
    translate_walk(va, walk(va))
}

fn translate_walk(va: usize, w: Walk) -> Option<HPA> {
    match w {
        Walk(Some(_), Some(L3E::Next(_)), Some(L2E::Next(_)), Some(L1E::Page(PF4K(hpa)))) => {
            Some(hpa.offset(va & <PF4K as PageFrame>::PageType::MASK))
        }
        Walk(Some(_), Some(L3E::Next(_)), Some(L2E::Page(PF2M(hpa))), _) => {
            Some(hpa.offset(va & <PF2M as PageFrame>::PageType::MASK))
        }
        Walk(Some(_), Some(L3E::Page(PF1G(hpa))), _, _) => {
            Some(hpa.offset(va & <PF1G as PageFrame>::PageType::MASK))
        }
        Walk(_, _, _, _) => None,
    }
}

/// Maps the given PF4K to the given virtual address in the current
/// address space.
pub fn map<F>(hpf: PF4K, flags: PTEFlags, va: V4KA, allocator: &mut F) -> Result<()>
where
    F: FnMut() -> Result<PF4K>,
{
    let va = va.addr();
    assert!(va < Level1::SIDE_BASE_ADDRESS, "attempting to map in the recursive region");

    let w = walk(va);
    if let Walk(None, _, _, _) = w {
        Level4::set_entry(va, alloc_inner(allocator)?);
    }
    if let Walk(_, None, _, _) = w {
        Level3::set_entry(va, alloc_inner(allocator)?);
    }
    if let Walk(_, _, None, _) = w {
        Level2::set_entry(va, alloc_inner(allocator)?);
    }
    if let Walk(_, _, _, None) = w {
        Level1::set_entry(va, PTE::new(hpf.pfa(), flags));
        Ok(())
    } else {
        Err("Already mapped")
    }
}

/// Maps a leaf node into the address space.  Requires that the
/// intermediate paging structures for the mapping already
/// exist.
pub fn map_leaf(hpf: PF4K, va: V4KA, r: bool, w: bool, x: bool) -> Result<()> {
    let flags = page_perm_flags(r, w, x);
    let mut allocator = || Err("not a leaf");
    map(hpf, flags, va, &mut allocator)
}

/// Unmaps the given virtual address in the current address space.
/// Only clears the leaf entry, ignoring interior nodes.
pub fn unmap(va: V4KA) {
    let va = va.addr();
    if let Walk(Some(_), Some(_), Some(_), Some(_)) = walk(va) {
        Level4::clear(va);
    }
}

// Converts RWX permissions to page flags.
fn page_perm_flags(r: bool, w: bool, x: bool) -> PTEFlags {
    let mut flags = PTEFlags::empty();
    if r {
        flags.insert(PTEFlags::PRESENT);
    }
    if w {
        flags.insert(PTEFlags::WRITE);
    }
    if !x {
        flags.insert(PTEFlags::NX);
    }
    flags
}

// Allocate an "inner" node in the radix tree; that is, make a
// new interior node that is, itself, a page table.
fn alloc_inner<A>(allocator: &mut A) -> Result<PTE>
where
    A: FnMut() -> Result<PF4K>,
{
    let inner_flags = PTEFlags::PRESENT | PTEFlags::WRITE;
    let p = allocator()?;
    Ok(PTE::new(p.pfa(), inner_flags))
}

// Makes the paging structures at a given level for the
// specified regions and page permissions.
fn make_ranges_level<L, F>(ranges: &[Range<V4KA>], allocator: &mut F) -> Result<()>
where
    F: FnMut() -> Result<PF4K>,
    L: Level,
{
    for range in ranges.iter() {
        let start = L::VPageAddrType::new_round_down(range.start.addr());
        let end = L::VPageAddrType::new_round_up(range.end.addr());
        assert!(
            end.addr() < Level1::SIDE_BASE_ADDRESS,
            "attempting to map in the recursive region"
        );
        for addr in start..end {
            let va = addr.addr();
            if L::entry(va).is_none() {
                let pf = allocator()?;
                L::set_entry(va, PTE::new(pf.pfa(), PTEFlags::WRITE | PTEFlags::PRESENT));
            }
        }
    }
    Ok(())
}

/// Creates paging structures corresponding to the given ranges
/// of addresses in the current address space.
///
/// Note this merely creates the structures but they do not point
/// to active pages after it completes.  It is assumed that the
/// allocator returns zeroed pages.
pub fn make_ranges<F>(ranges: &[Range<V4KA>], allocator: &mut F) -> Result<()>
where
    F: FnMut() -> Result<PF4K>,
{
    make_ranges_level::<Level4, _>(ranges, allocator)?;
    make_ranges_level::<Level3, _>(ranges, allocator)?;
    make_ranges_level::<Level2, _>(ranges, allocator)?;
    Ok(())
}

/// Creates paging structures corresponding to the given
/// ranges of addresses in both the current and side-loaded
/// address spaces, pointing to empty pages.
///
/// It is assumed that the allocator returns zeroed pages.
pub fn make_shared_ranges<A>(ranges: &[Range<V4KA>], side: PF4K, allocator: &mut A) -> Result<PF4K>
where
    A: FnMut() -> Result<PF4K>,
{
    fn make_shared_ranges_level4<A>(ranges: &[Range<V4KA>], allocator: &mut A) -> Result<()>
    where
        A: FnMut() -> Result<PF4K>,
    {
        for range in ranges {
            let start = V512GA::new_round_down(range.start.addr());
            let end = V512GA::new_round_up(range.end.addr());
            assert!(
                end.addr() < Level1::SIDE_BASE_ADDRESS,
                "attempting to map in the recursive region"
            );
            for addr in start..end {
                let va = addr.addr();
                let entry = Level4::pte_ref(va);
                if entry.is_zero() {
                    let pf = allocator()?;
                    entry.assign(PTE::new(pf.pfa(), PTEFlags::WRITE | PTEFlags::PRESENT));
                }
                unsafe {
                    Level4::set_side_entry(va, entry.clone());
                }
            }
        }
        Ok(())
    }
    unsafe {
        side_load(side)?;
    }
    make_shared_ranges_level4::<_>(ranges, allocator)?;
    make_ranges_level::<Level3, _>(ranges, allocator)?;
    make_ranges_level::<Level2, _>(ranges, allocator)?;
    unload_side()
}

/// Shares some subtree of an address space into a side-loaded
/// space.
pub fn share_range<A>(range: Range<V4KA>, side: PF4K, allocator: &mut A) -> Result<PF4K>
where
    A: FnMut() -> Result<PF4K>,
{
    const SIZE_512G: usize = <V512GA as VPageAddr>::PageType::SIZE;
    const SIZE_1G: usize = <V1GA as VPageAddr>::PageType::SIZE;
    const SIZE_2M: usize = <V2MA as VPageAddr>::PageType::SIZE;
    const SIZE_4K: usize = <V4KA as VPageAddr>::PageType::SIZE;

    let mut va = range.start.addr();
    let end = range.end.addr();
    assert!(end <= Level1::SIDE_BASE_ADDRESS, "attempting to map in the recursive region");
    unsafe {
        side_load(side)?;
    }
    while va != end {
        let len = if end.wrapping_sub(va) >= SIZE_512G && va % SIZE_512G == 0 {
            unsafe {
                Level4::set_side_entry(va, Level4::pte_ref(va).clone());
            }
            SIZE_512G
        } else if end.wrapping_sub(va) >= SIZE_1G && va % SIZE_1G == 0 {
            unsafe {
                Level4::make_side_level(V4KA::new(va), allocator)?;
                Level3::set_side_entry(va, Level3::pte_ref(va).clone());
            }
            SIZE_1G
        } else if end.wrapping_sub(va) >= SIZE_2M && va % SIZE_2M == 0 {
            unsafe {
                Level3::make_side_level(V4KA::new(va), allocator)?;
                Level2::set_side_entry(va, Level2::pte_ref(va).clone());
            }
            SIZE_2M
        } else if end.wrapping_sub(va) >= SIZE_4K && va % SIZE_4K == 0 {
            unsafe {
                Level2::make_side_level(V4KA::new(va), allocator)?;
                Level1::set_side_entry(va, Level1::pte_ref(va).clone());
            }
            SIZE_4K
        } else {
            panic!("impossible page size");
        };
        va += len;
    }
    unload_side()
}

/// unmaps a region by clearing its root level PTEs.  Only
/// useful for segments and tasks.
pub fn unmap_root_ranges(ranges: &[Range<V4KA>]) {
    let _tlb = TLBFlushGuard::new();
    for range in ranges {
        let start = V512GA::new_round_down(range.start.addr());
        let end = V512GA::new_round_up(range.end.addr());
        for addr in start..end {
            let entry = Level4::pte_ref(addr.addr());
            entry.clear();
        }
    }
}

/// unmaps a side region by clearing its root level PTEs.  Only
/// useful for segments and tasks.
///
/// # Safety
/// This is not safe.  The side-loaded address space may not
/// be loaded.
pub unsafe fn unmap_side_root_ranges(ranges: &[Range<V4KA>]) {
    let _tlb = TLBFlushGuard::new();
    for range in ranges {
        let start = V512GA::new_round_down(range.start.addr());
        let end = V512GA::new_round_up(range.end.addr());
        for addr in start..end {
            let entry = unsafe { Level4::side_pte_ref(addr.addr()) };
            entry.clear();
        }
    }
}

/// Maps an address space in the side-load slot.
///
/// # Safety
///
/// This is not safe.  The side-loaded "address space" may not
/// be an address space at all.
pub unsafe fn side_load(pf: PF4K) -> Result<()> {
    let _tlb = TLBFlushGuard::new();
    let table = unsafe { &mut *PageTable::proto_ptr().with_addr(Level4::BASE_ADDRESS).cast_mut() };
    table.entries[Level4::SIDE_INDEX] = PTE::new(pf.pfa(), PTEFlags::PRESENT | PTEFlags::WRITE);
    Ok(())
}

/// Unmaps a side-loaded address space.
///
/// # Safety
///
/// This is not safe.  The side-loaded address space may not
/// be loaded.
pub fn unload_side() -> Result<PF4K> {
    let _tlb = TLBFlushGuard::new();
    let table = unsafe { &mut *PageTable::proto_ptr().with_addr(Level4::BASE_ADDRESS).cast_mut() };
    let entry = table.entries[Level4::SIDE_INDEX].pfa();
    table.entries[Level4::SIDE_INDEX].clear();
    Ok(PF4K::new(entry))
}

/// Performs a TLB flush on the local CPU.
pub fn flush_tlb() {
    unsafe {
        let cr3 = x86::controlregs::cr3();
        x86::controlregs::cr3_write(cr3);
    }
}

/// Perform a walk against a side-loaded page table.
///
/// # Safety
///
/// This is not safe.  The caller must ensure that a side-loaded
/// page table is loaded, and that the TLB is free of stale entries
/// for any other side-loaded address space before calling this.
///
/// XXX(cross): We should figure out some way to at least improve
/// safety here.
unsafe fn side_walk(va: usize) -> Walk {
    let pt4e = unsafe { Level4::side_entry(va) };
    match pt4e {
        Some(_) => {}
        _ => return Walk(pt4e, None, None, None),
    }

    let pt3e = unsafe { Level3::side_entry(va) };
    match pt3e {
        Some(L3E::Next(_)) => {}
        _ => return Walk(pt4e, pt3e, None, None),
    }

    let pt2e = unsafe { Level2::side_entry(va) };
    match pt2e {
        Some(L2E::Next(_)) => {}
        _ => return Walk(pt4e, pt3e, pt2e, None),
    }

    let pt1e = unsafe { Level1::side_entry(va) };

    Walk(pt4e, pt3e, pt2e, pt1e)
}

/// Translate a given virtual address into a host physical
/// address against the currently side-loaded page table.
///
/// # Safety
///
/// This is not safe.  The caller must ensure that a side-loaded
/// page table is loaded, and that the TLB is free of stale entries
/// for any other side-loaded address space before calling this.
///
/// XXX(cross): We should figure out some way to at least improve
/// safety here.
pub unsafe fn side_translate(va: usize) -> Option<HPA> {
    translate_walk(va, unsafe { side_walk(va) })
}

/// Maps the given PF4K to the given virtual address in the currently
/// side-loaded address space.
///
/// # Safety
///
/// This is not safe.  The caller must ensure that a side-loaded
/// page table is mapped, and that the TLB is free of stale entries.
pub unsafe fn side_map<F>(hpf: PF4K, flags: PTEFlags, va: V4KA, allocator: &mut F) -> Result<()>
where
    F: FnMut() -> Result<PF4K>,
{
    let va = va.addr();
    let w = unsafe { side_walk(va) };
    if let Walk(None, _, _, _) = w {
        unsafe {
            Level4::set_side_entry(va, alloc_inner(allocator)?);
        }
    }
    if let Walk(_, None, _, _) = w {
        unsafe {
            Level3::set_side_entry(va, alloc_inner(allocator)?);
        }
    }
    if let Walk(_, _, None, _) = w {
        unsafe {
            Level2::set_side_entry(va, alloc_inner(allocator)?);
        }
    }
    if let Walk(_, _, _, None) = w {
        unsafe {
            Level1::set_side_entry(va, PTE::new(hpf.pfa(), flags));
        }
        Ok(())
    } else {
        Err("Already side mapped")
    }
}

/// Returns the host physical address of the address space root for
/// the currently loaded address space.
pub fn address_space_root() -> HPA {
    let table = unsafe { &*PageTable::proto_ptr().with_addr(Level4::BASE_ADDRESS) };
    table.root_addr()
}

struct TLBFlushGuard {}
impl TLBFlushGuard {
    pub fn new() -> TLBFlushGuard {
        TLBFlushGuard {}
    }
}
impl Drop for TLBFlushGuard {
    fn drop(&mut self) {
        flush_tlb();
    }
}

#[cfg(test)]
mod tests {
    use super::Level;

    #[test]
    fn level4_base() {
        use super::Level4;
        let base = !0usize << 48;
        let base = base | Level4::SELF_INDEX << 39;
        let base = base | Level4::SELF_INDEX << 30;
        let base = base | Level4::SELF_INDEX << 21;
        let side = base | Level4::SIDE_INDEX << 12;
        let base = base | Level4::SELF_INDEX << 12;
        assert_eq!(side, Level4::SIDE_BASE_ADDRESS);
        assert_eq!(base, Level4::BASE_ADDRESS);
    }

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
        assert_eq!(Level4::index(Level4::BASE_ADDRESS), 511);
        assert_eq!(Level4::index(Level4::SIDE_BASE_ADDRESS), 511);
    }

    #[test]
    fn level3_base() {
        use super::Level4;
        let base = !0usize << 48;
        let base = base | Level4::SELF_INDEX << 39;
        let base = base | Level4::SELF_INDEX << 30;
        let side = base | Level4::SIDE_INDEX << 21;
        let base = base | Level4::SELF_INDEX << 21;
        assert_eq!(side, super::Level3::SIDE_BASE_ADDRESS);
        assert_eq!(base, super::Level3::BASE_ADDRESS);
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
    fn level2_base() {
        use super::Level4;
        let base = !0usize << 48;
        let base = base | Level4::SELF_INDEX << 39;
        let side = base | Level4::SIDE_INDEX << 30;
        let base = base | Level4::SELF_INDEX << 30;
        assert_eq!(side, super::Level2::SIDE_BASE_ADDRESS);
        assert_eq!(base, super::Level2::BASE_ADDRESS);
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
    fn level1_base() {
        use super::Level4;
        let base = !0usize << 48;
        let side = base | Level4::SIDE_INDEX << 39;
        let base = base | Level4::SELF_INDEX << 39;
        assert_eq!(side, super::Level1::SIDE_BASE_ADDRESS);
        assert_eq!(base, super::Level1::BASE_ADDRESS);
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

    #[test]
    fn pte_debug() {
        use super::{HPA, PTE, PTEFlags as F};

        let pte = PTE::new(HPA::new(0xabc000), F::NX | F::USER | F::WRITE | F::PRESENT);
        assert_eq!(format!("{:?}", pte), "-:0xabc000:-----UWR");

        let pte = PTE::new(HPA::new(0xfff000), F::NOCACHE | F::USER | F::WRITE | F::PRESENT);
        assert_eq!(format!("{:?}", pte), "X:0xfff000:----C̶UWR");
    }
}
