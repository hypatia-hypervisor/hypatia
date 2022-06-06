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

use arch::io::Sender;

libhypatia::define_task!();

#[cfg_attr(not(test), start, no_mangle)]
pub extern "C" fn main() {
    let mut port = arch::io::OutPort::new(0x3f8);
    port.send(b'a');
}
