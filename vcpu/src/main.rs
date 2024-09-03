// Copyright 2021  The Hypatia Authors
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

use arch::io::Sender;

#[unsafe(no_mangle)]
pub extern "C" fn start() {
    let mut port = arch::io::OutPort::new(0x3f8);
    port.send(b'a');
}

hypatia::runtime!();
