// Copyright 2022  The Hypatia Authors
// All rights reserved
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//
// Minimal println for debugging directly in the arch crate.
//
pub struct Uart {}
impl core::fmt::Write for Uart {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        fn putb(b: u8) {
            use crate::io::{Receiver, Sender};
            use bit_field::BitField;
            const EIA0: u16 = 0x3f8;
            fn tx_ready() -> bool {
                let mut lsr = crate::io::InPort::<u8>::new(EIA0 + 5);
                let b = lsr.recv();
                b.get_bit(5)
            }
            while !tx_ready() {
                crate::cpu::relax();
            }
            let mut thr = crate::io::OutPort::new(EIA0);
            thr.send(b);
        }
        for b in s.bytes() {
            if b == b'\n' {
                putb(b'\r');
            }
            putb(b);
        }
        Ok(())
    }
}

// These macros do not lock, so that they can be called from
// a panic!() handler on a potentially wedged machine.
#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[macro_export]
macro_rules! print {
    ($($args:tt)*) => ({
        use core::fmt::Write;
        let mut uart = $crate::debug::Uart {};
        uart.write_fmt(format_args!($($args)*)).unwrap();
    })
}
