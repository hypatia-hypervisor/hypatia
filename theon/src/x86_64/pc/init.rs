// Copyright 2021  The Hypatia Authors
// All rights reserved
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use crate::x86_64::pc::multiboot1;
use core::cell::SyncUnsafeCell;
use core::sync::atomic::{AtomicBool, Ordering};

pub(crate) fn start(mbinfo_phys: u64) -> multiboot1::Multiboot1 {
    static INITED: AtomicBool = AtomicBool::new(false);
    if INITED.swap(false, Ordering::SeqCst) {
        panic!("double init");
    }
    static IDT: SyncUnsafeCell<arch::idt::IDT> = SyncUnsafeCell::new(arch::idt::IDT::empty());
    static GDT: SyncUnsafeCell<arch::gdt::GDT> = SyncUnsafeCell::new(arch::gdt::GDT::empty());
    static TSS: SyncUnsafeCell<arch::tss::TSS> = SyncUnsafeCell::new(arch::tss::TSS::empty());

    uart::panic_println!("\nBooting Hypatia...");
    let idt = unsafe { &mut *IDT.get() };
    idt.init(arch::trap::stubs());
    unsafe {
        arch::idt::load(idt);
    }
    let tss = unsafe { &*TSS.get() };
    let gdt = unsafe { &mut *GDT.get() };
    gdt.init(tss);
    unsafe {
        arch::gdt::load(gdt);
    }
    multiboot1::init(mbinfo_phys)
}
