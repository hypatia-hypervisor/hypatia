// Copyright 2023  The Hypatia Authors
// All rights reserved
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use arch::idt;
use core::sync::atomic::{AtomicBool, Ordering};

static mut IDT: idt::IDT = idt::IDT::empty();
static INITED: AtomicBool = AtomicBool::new(false);

pub(crate) fn init() {
    if !INITED.swap(true, Ordering::AcqRel) {
        unsafe {
            IDT.init(arch::trap::stubs());
        }
    }
}
