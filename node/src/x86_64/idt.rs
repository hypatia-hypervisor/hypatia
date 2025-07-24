// Copyright 2023  The Hypatia Authors
// All rights reserved
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use core::cell::SyncUnsafeCell;
use core::sync::atomic::{AtomicBool, Ordering};

static IDT: SyncUnsafeCell<arch::idt::IDT> = SyncUnsafeCell::new(arch::idt::IDT::empty());
static INITED: AtomicBool = AtomicBool::new(false);

pub(crate) fn init() {
    if INITED.swap(true, Ordering::AcqRel) {
        panic!("double init node IDT");
    }
    let idt = unsafe { &mut *IDT.get() };
    idt.init(arch::trap::stubs());
    unsafe {
        arch::idt::load(idt);
    }
}
