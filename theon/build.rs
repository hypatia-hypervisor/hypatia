// Copyright 2021  The Hypatia Authors
// All rights reserved
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use std::env;

fn main() {
    let target = env::var("TARGET").unwrap();
    if target.as_str() == "x86_64-unknown-none-elf" {
        println!("cargo:rustc-link-arg-bins=--script=theon/src/x86_64/pc/link.ld")
    }
}
