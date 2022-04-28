// Copyright 2021  The Hypatia Authors
// All rights reserved
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

#![feature(lang_items)]
#![feature(start)]
#![feature(strict_provenance)]
#![cfg_attr(not(test), no_main)]
#![cfg_attr(not(test), no_std)]

libhypatia::define_segment!(init);

// XXX(mikew): For some reason, removing this no_mangle on this init in particular causes
// initialization to hang.
#[no_mangle]
fn init() {
    uart::panic_println!("Hi from the monitor");
}
