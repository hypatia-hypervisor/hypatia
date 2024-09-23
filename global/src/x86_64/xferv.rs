// Copyright 2023  The Hypatia Authors
// All rights reserved
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use core::arch::asm;

#[export_name = "xferv"]
#[link_section = ".xferv"]
#[naked]
unsafe extern "C" fn xferv() {
    unsafe {
        asm!(r#"
            .balign 8; jmp {hi};
            .balign 8; jmp {bye};
            "#,
            hi = sym hi,
            bye = sym bye,
            options(att_syntax, noreturn));
    }
}

extern "C" fn hi() {
    uart::panic_println!("Hi!");
}

extern "C" fn bye() {
    uart::panic_println!("Bye!");
}
