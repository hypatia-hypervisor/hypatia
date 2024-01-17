// Copyright 2021  The Hypatia Authors
// All rights reserved
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicUsize, Ordering};

/// The allocator works in terms of an owned region
/// of memory.  We call this a Heap.
pub(crate) trait Heap {
    fn as_mut_ptr(&mut self) -> *mut u8;
    fn len(&self) -> usize;
}

/// A SliceHeap is a heap created by destructuring
/// the elements of a mutable slice.
pub(crate) struct SliceHeap {
    heap: *mut u8,
    len: usize,
}

impl SliceHeap {
    pub fn new(arena: &mut [u8]) -> SliceHeap {
        SliceHeap { heap: arena.as_mut_ptr(), len: arena.len() }
    }
}
impl Heap for SliceHeap {
    fn as_mut_ptr(&mut self) -> *mut u8 {
        self.heap
    }
    fn len(&self) -> usize {
        self.len
    }
}

/// A Bump Allocator takes ownership of an object of
/// some type that implements Heap, and maintains a
/// cursor into that object.  The cursor denotes the
/// point between allocated and unallocated memory in
/// the underlying Heap.
pub(crate) struct BumpAlloc<T: Heap> {
    arena: UnsafeCell<T>,
    cursor: AtomicUsize,
}

impl<T: Heap> BumpAlloc<T> {
    pub(crate) const fn new(arena: T) -> BumpAlloc<T> {
        BumpAlloc { arena: UnsafeCell::new(arena), cursor: AtomicUsize::new(0) }
    }

    /// Allocates the given number of bytes with the given
    /// alignment.  Returns `None` if the allocation cannot
    /// be satisfied, otherwise returns `Some` of a mutable
    /// slice referring to the allocated memory.
    pub(crate) fn alloc_bytes(&self, align: usize, size: usize) -> Option<&mut [u8]> {
        let heap = unsafe { &mut *self.arena.get() };
        let base = heap.as_mut_ptr();
        let mut offset = 0;
        self.cursor
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
                let ptr = base.wrapping_add(current);
                let adjust = ptr.align_offset(align);
                offset = current.checked_add(adjust).expect("alignment overflow");
                let next = offset.checked_add(size).expect("size overflow");
                (next <= heap.len()).then_some(next)
            })
            .ok()?;
        let ptr = base.wrapping_add(offset);
        Some(unsafe { core::slice::from_raw_parts_mut(ptr, size) })
    }
}

mod global {
    use super::{BumpAlloc, Heap};
    use alloc::alloc::{GlobalAlloc, Layout};
    use core::ptr;

    const GLOBAL_HEAP_SIZE: usize = 4 * 1024 * 1024;

    /// A GlobalHeap is an aligned wrapper around an
    /// owned buffer that implements the Heap trait.
    #[repr(C, align(4096))]
    struct GlobalHeap([u8; GLOBAL_HEAP_SIZE]);

    impl Heap for GlobalHeap {
        fn as_mut_ptr(&mut self) -> *mut u8 {
            self.0.as_mut_ptr()
        }
        fn len(&self) -> usize {
            GLOBAL_HEAP_SIZE
        }
    }

    unsafe impl<T: Heap> GlobalAlloc for BumpAlloc<T> {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            self.alloc_bytes(layout.align(), layout.size())
                .map_or(ptr::null_mut(), |p| p.as_mut_ptr())
        }
        unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
    }

    #[global_allocator]
    static mut BUMP_ALLOCATOR: BumpAlloc<GlobalHeap> =
        BumpAlloc::new(GlobalHeap([0u8; GLOBAL_HEAP_SIZE]));
}
