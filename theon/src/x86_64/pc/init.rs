// Copyright 2021  The Hypatia Authors
// All rights reserved
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use crate::x86_64::pc::multiboot1;
use core::ptr;
use core::sync::atomic::{AtomicBool, Ordering};

pub(crate) fn start(mbinfo_phys: u64) -> multiboot1::Multiboot1 {
    static INITED: AtomicBool = AtomicBool::new(false);
    if INITED.swap(false, Ordering::SeqCst) {
        panic!("double init");
    }
    static mut IDT: arch::idt::IDT = arch::idt::IDT::empty();
    static mut GDT: arch::gdt::GDT = arch::gdt::GDT::empty();
    static mut TSS: arch::tss::TSS = arch::tss::TSS::empty();

    uart::panic_println!("\nBooting Hypatia...");
    unsafe {
        IDT.init(arch::trap::stubs());
        arch::idt::load(&mut *ptr::addr_of_mut!(IDT));
        GDT.init(&*ptr::addr_of!(TSS));
        arch::gdt::load(&mut *ptr::addr_of_mut!(GDT));
    }
    multiboot1::init(mbinfo_phys)
}
