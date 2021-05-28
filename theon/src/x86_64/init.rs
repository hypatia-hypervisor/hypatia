// Copyright 2021  The Hypatia Authors
// All rights reserved
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use crate::x86_64::multiboot1;

pub fn start(mbinfo_phys: u64) {
    uart::panic_println!("\nBooting Hypatia...");
    multiboot1::init(mbinfo_phys);
}
