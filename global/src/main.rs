// Copyright 2023  The Hypatia Authors
// All rights reserved
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

#![cfg_attr(not(test), no_main)]
#![cfg_attr(not(test), no_std)]
#![forbid(absolute_paths_not_starting_with_crate)]
#![forbid(elided_lifetimes_in_paths)]
#![forbid(unsafe_op_in_unsafe_fn)]

mod x86_64;

use arch::Page4K;

/// Returns a static reference to the global zero page.
pub fn zero_page() -> &'static Page4K {
    const ZERO_PAGE: Page4K = Page4K::new();
    &ZERO_PAGE
}

/// Initialize the system.
#[unsafe(no_mangle)]
pub extern "C" fn init() {
    zero_page();
    uart::panic_println!("Hello from global");
}

hypatia::runtime!();
