// Copyright 2021  The Hypatia Authors
// All rights reserved
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

#![feature(strict_provenance)]
#![cfg_attr(not(test), no_std)]

mod x86_64;

pub mod arch {
    pub use crate::x86_64::ns16550::*;
}

// These macros do not lock, so that they can be called from
// a panic!() handler on a potentially wedged machine.
#[macro_export]
macro_rules! panic_println {
    () => (uart_print!("\n"));
    ($($arg:tt)*) => ($crate::panic_print!("{}\n", format_args!($($arg)*)));
}

#[macro_export]
macro_rules! panic_print {
    ($($args:tt)*) => ({
        use core::fmt::Write;
        let mut uart = $crate::arch::Uart::new($crate::arch::Port::Eia0);
        uart.write_fmt(format_args!($($args)*)).unwrap();
    })
}
