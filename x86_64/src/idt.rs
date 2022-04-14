// Copyright 2021  The Hypatia Authors
// All rights reserved
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use crate::StackIndex;
use crate::{segment, trap};

#[repr(C, align(4096))]
pub struct IDT {
    entries: [segment::InterruptGateDescriptor; 256],
}

fn make_gate(thunk: &trap::Stub, vecnum: u8) -> segment::InterruptGateDescriptor {
    const NMI_STACK: StackIndex = StackIndex::Ist1;
    const DEBUG_STACK: StackIndex = StackIndex::Ist2;
    const DOUBLE_FAULT_STACK: StackIndex = StackIndex::Ist3;
    match vecnum {
        1 => segment::InterruptGateDescriptor::new(thunk, DEBUG_STACK),
        2 => segment::InterruptGateDescriptor::new(thunk, NMI_STACK),
        8 => segment::InterruptGateDescriptor::new(thunk, DOUBLE_FAULT_STACK),
        _ => segment::InterruptGateDescriptor::new(thunk, StackIndex::Rsp0),
    }
}

impl IDT {
    /// Returns an empty IDT.
    pub const fn empty() -> IDT {
        IDT { entries: [segment::InterruptGateDescriptor::empty(); 256] }
    }

    /// # Safety
    ///
    /// Called once for every IDT.
    pub unsafe fn init(idt: *mut IDT, stubs: &[trap::Stub; 256]) {
        let entries = idt as *mut segment::InterruptGateDescriptor;
        for (k, stub) in stubs.iter().enumerate() {
            let gate = make_gate(stub, k as u8);
            core::ptr::write_volatile(entries.add(k), gate);
        }
    }

    /// Loads the IDT into the processor.
    ///
    /// # Safety
    ///
    /// Early code assumes a good stack and resets the IDT pointer
    /// on the local processor.
    pub unsafe fn load(&mut self) {
        let base = (self as *mut Self).addr() as u64;
        const LIMIT: u16 = (core::mem::size_of::<IDT>() - 1) as u16;
        core::arch::asm!(r#"
            subq $16, %rsp;
            movq {base}, 8(%rsp);
            movw ${LIMIT}, 6(%rsp);
            lidt 6(%rsp);
            addq $16, %rsp;
            "#,
            base = in(reg) base, LIMIT = const LIMIT,
            options(att_syntax));
    }
}
