// Copyright 2023  The Hypatia Authors
// All rights reserved
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use arch::{Page, V4KA, VPageAddr, gdt};
use core::sync::atomic::{AtomicBool, Ordering};

#[unsafe(link_section = ".gdt")]
static mut GDT: gdt::GDT = gdt::GDT::empty();
static INITED: AtomicBool = AtomicBool::new(false);

pub(crate) fn map() {
    let zeros = crate::zero_page();
    let va = V4KA::new((&raw const GDT).addr());
    const R: bool = true;
    const NW: bool = false;
    const NX: bool = false;
    for k in 1..16 {
        let zva = V4KA::new(va.addr() + k * 4096);
        arch::vm::map_leaf(zeros.frame(), zva, R, NW, NX).expect("mapped zero page in GDT");
    }
}

pub(crate) fn init(task_state: &arch::tss::TSS) {
    if !INITED.swap(true, Ordering::AcqRel) {
        let gdtp = &raw mut GDT;
        let gdt = unsafe { &mut *gdtp };
        gdt.init(task_state);
        unsafe {
            arch::gdt::load(gdt);
        }
    }
}
