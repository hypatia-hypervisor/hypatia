// Copyright 2021  The Hypatia Authors
// All rights reserved
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

#![feature(lang_items)]
#![feature(start)]
#![cfg_attr(not(test), no_main)]
#![cfg_attr(not(test), no_std)]

mod x86_64;

#[cfg_attr(not(test), start, no_mangle)]
pub extern "C" fn main() {
    unsafe {
        core::arch::asm!("movl $0xcafef00d, %eax", options(att_syntax));
    }
}

#[cfg(not(test))]
mod runtime;
