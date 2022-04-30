// Copyright 2021  The Hypatia Authors
// All rights reserved
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use crate::x86_64::multiboot1;

static mut IDT: arch::idt::IDT = arch::idt::IDT::empty();
static mut GDT: arch::gdt::GDT = arch::gdt::GDT::empty();
static mut TSS: arch::tss::TSS = arch::tss::TSS::empty();

pub(crate) fn start(mbinfo_phys: u64) -> multiboot1::Multiboot1 {
    uart::panic_println!("\nBooting Hypatia...");
    unsafe {
        arch::idt::IDT::init(&mut IDT, arch::trap::stubs());
        arch::idt::IDT::load(&IDT);
        GDT = arch::gdt::GDT::new(&TSS);
        GDT.load();
    }
    multiboot1::init(mbinfo_phys)
}
