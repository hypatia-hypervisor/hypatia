// Copyright 2021  The Hypatia Authors
// All rights reserved
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

#![feature(start)]
#![feature(strict_provenance)]
#![cfg_attr(not(test), no_main)]
#![cfg_attr(not(test), no_std)]
#![forbid(unsafe_op_in_unsafe_fn)]

libhypatia::define_segment!(init);

use arch::Page4K;

/// Returns a static reference to the global zero page.
pub fn zero_page() -> &'static Page4K {
    const ZERO_PAGE: Page4K = Page4K::new();
    &ZERO_PAGE
}

/// Initialize the system.
fn init() {
    zero_page();
}
