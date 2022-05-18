// Copyright 2021  The Hypatia Authors
// All rights reserved
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! Task State Segment support.
//!
//! In the host, we only support the 64-bit TSS.

use crate::segment;
use crate::tss::TSS;
use core::arch::asm;

/// Support the x86_64 64-bit Global Descriptor Table.
///
/// We waste a few bytes per CPU by allocating a 4KiB page for
/// the GDT, then we map that at the known GDT location in the
/// per-CPU virtual memory segment, but we pad that out to 64KiB
/// by mapping the zero page repeatedly beyond the end of the
/// GDT proper.
///
/// We do this, in part, because VMX unconditionally resets the
/// limit on the GDT to 65535 on VM exit and we don't want to
/// reset the segment descriptor each time, nor have hardware
/// accidentally doing strange things because of segmentation.
#[repr(C, align(4096))]
pub struct GDT {
    null: segment::Descriptor,
    hypertext: segment::Descriptor,
    _hyperdata: segment::Descriptor,
    _userdata: segment::Descriptor,
    _usertext: segment::Descriptor,
    _unused: segment::Descriptor, // For alignment.
    task: segment::TaskStateDescriptor,
}

impl GDT {
    pub const fn empty() -> GDT {
        GDT {
            null: segment::Descriptor::empty(),
            hypertext: segment::Descriptor::empty(),
            _hyperdata: segment::Descriptor::empty(),
            _userdata: segment::Descriptor::empty(),
            _usertext: segment::Descriptor::empty(),
            _unused: segment::Descriptor::empty(),
            task: segment::TaskStateDescriptor::empty(),
        }
    }

    /// Returns a new GDT with a task segment descriptor that refers
    /// to the given TSS.
    pub fn new(task_state: &TSS) -> GDT {
        GDT {
            null: segment::Descriptor::null(),
            hypertext: segment::Descriptor::code64(),
            _hyperdata: segment::Descriptor::empty(),
            _userdata: segment::Descriptor::empty(),
            _usertext: segment::Descriptor::empty(),
            _unused: segment::Descriptor::empty(),
            task: task_state.descriptor(),
        }
    }

    /// Returns the code selector for %cs
    pub const fn code_selector() -> u16 {
        1 << 3
    }

    /// Returns the task selector for %tr
    pub const fn task_selector() -> u16 {
        6 << 3
    }

    /// Loads the GDTR with this GDT by building a descriptor on the
    /// stack and then invoking the LGDT instruction on that descriptor.
    ///
    /// # Safety
    ///
    /// Called on a valid GDT.
    unsafe fn lgdt(&self) {
        let base = u64::try_from((self as *const Self).addr()).unwrap();
        const LIMIT: u16 = core::mem::size_of::<GDT>() as u16 - 1;
        unsafe {
            asm!(r#"
                subq $16, %rsp;
                movq {}, 8(%rsp);
                movw ${}, 6(%rsp);
                lgdt 6(%rsp);
                addq $16, %rsp;
                "#, in(reg) base, const LIMIT, options(att_syntax));
        }
    }

    /// Loads the %tr register with a selector referring to a GDT's
    /// TSS descriptor.
    ///
    /// # Safety
    ///
    /// Private function that's called from a public function that
    /// ensures that a valid GDT with a task descriptor in the correct
    /// position is loaded before this is invoked.
    unsafe fn ltr(selector: u16) {
        unsafe {
            asm!("ltr {:x};", in(reg) selector);
        }
    }

    /// Loads this GDT and sets the task register to refer to its
    /// TSS descriptor.
    ///
    /// # Safety
    ///
    /// Must be called on a valid, initialized GDT.
    pub unsafe fn load(&self) {
        unsafe {
            self.lgdt();
            Self::ltr(Self::task_selector());
        }
    }
}
