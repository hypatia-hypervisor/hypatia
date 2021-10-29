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

fn make_gate(thunk: unsafe extern "C" fn() -> !, vecnum: u8) -> segment::InterruptGateDescriptor {
    const NMI_STACK: StackIndex = StackIndex::Ist1;
    const DEBUG_STACK: StackIndex = StackIndex::Ist2;
    const DOUBLE_FAULT_STACK: StackIndex = StackIndex::Ist3;
    match vecnum {
        2 => segment::InterruptGateDescriptor::new(thunk, NMI_STACK),
        3 => segment::InterruptGateDescriptor::new(thunk, DEBUG_STACK),
        8 => segment::InterruptGateDescriptor::new(thunk, DOUBLE_FAULT_STACK),
        _ => segment::InterruptGateDescriptor::new(thunk, StackIndex::Rsp0),
    }
}

impl IDT {
    /// # Safety
    ///
    /// Called once for every IDT.
    pub unsafe fn init(idt: *mut IDT, stubs: &[trap::Stub; 256]) {
        let entries = idt as *mut segment::InterruptGateDescriptor;
        for (k, stub) in stubs.iter().enumerate() {
            let gate = make_gate(*stub, k as u8);
            core::ptr::write_volatile(entries.add(k), gate);
        }
    }
}
