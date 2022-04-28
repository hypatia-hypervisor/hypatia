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

libhypatia::define_task!();

#[start]
pub extern "C" fn main() {}
