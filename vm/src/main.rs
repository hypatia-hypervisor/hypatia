// Copyright 2021  The Hypatia Authors
// All rights reserved
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

#![feature(strict_provenance)]
#![cfg_attr(not(test), no_main)]
#![cfg_attr(not(test), no_std)]
#![forbid(unsafe_op_in_unsafe_fn)]

mod x86_64;

libhypatia::define_task!();

#[no_mangle]
pub extern "C" fn start() {
    unsafe {
        core::arch::asm!("movl $0xcafef00d, %eax", options(att_syntax));
    }
}
