// Copyright 2021  The Hypatia Authors
// All rights reserved
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! # Panic Utility Functions
//!
//! This module includes some utility functions useful for implementing panics in tasks.

use core::panic::PanicInfo;

/// Print a `PanicInfo` struct out to the console.
pub fn print_panic(info: &PanicInfo<'_>) {
    uart::panic_println!("\nPANIC: ");
    uart::panic_println!("*************** [ Cut Here ] *************");
    uart::panic_println!("{:#?}", info);
    uart::panic_println!("******************************************");
    uart::panic_println!("System halted.");
}
