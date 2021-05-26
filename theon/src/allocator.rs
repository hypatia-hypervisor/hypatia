extern crate alloc;
use alloc::alloc::{GlobalAlloc, Layout};
use core::cell::Cell;

static mut HEAP: [u8; 4 * 1024 * 1024] = [0_u8; 4 * 1024 * 1024];

struct BumpAlloc<'a> {
    heap: Cell<&'a mut [u8]>,
}

unsafe impl GlobalAlloc for BumpAlloc<'_> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let heap = self.heap.take();
        let ptr = heap.as_mut_ptr();
        let offset = ptr.align_offset(layout.align());
        if offset > heap.len() || offset + layout.size() > heap.len() {
            return core::ptr::null_mut();
        }
        let ptr = ptr.add(offset);
        let heap = &mut heap[offset + layout.size()..];
        self.heap.replace(heap);
        ptr
    }
    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
}

#[global_allocator]
static mut BUMP_ALLOCATOR: BumpAlloc = BumpAlloc {
    heap: Cell::new(unsafe { &mut HEAP }),
};
