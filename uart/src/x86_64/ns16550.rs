// Copyright 2021  The Hypatia Authors
// All rights reserved
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use arch::io::{Receiver, Sender};
use bit_field::BitField;
use core::fmt;

pub enum Port {
    Eia0,
    Eia1,
}

pub struct Uart(u16);

impl Uart {
    pub fn new(port: Port) -> Uart {
        match port {
            Port::Eia0 => Uart(0x3f8),
            Port::Eia1 => Uart(0x2f8),
        }
    }

    fn lsr(&mut self) -> arch::io::InPort<u8> {
        arch::io::InPort::new(self.0 + 5)
    }

    fn thr(&mut self) -> arch::io::OutPort<u8> {
        arch::io::OutPort::new(self.0)
    }

    fn rbr(&mut self) -> arch::io::InPort<u8> {
        arch::io::InPort::new(self.0)
    }

    fn tx_ready(&mut self) -> bool {
        let mut lsr = self.lsr();
        let b = lsr.recv();
        b.get_bit(5)
    }

    pub fn putb(&mut self, b: u8) {
        while !self.tx_ready() {
            arch::cpu::relax();
        }
        self.thr().send(b);
    }

    pub fn puts(&mut self, s: &str) {
        for b in s.bytes() {
            self.putb(b);
        }
    }

    pub fn rx_ready(&mut self) -> bool {
        let mut lsr = self.lsr();
        let b = lsr.recv();
        b.get_bit(0)
    }

    pub fn getb(&mut self) -> u8 {
        while !self.rx_ready() {
            arch::cpu::relax();
        }
        self.rbr().recv()
    }
}

impl fmt::Write for Uart {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for b in s.bytes() {
            if b == b'\n' {
                self.putb(b'\r');
            }
            self.putb(b);
        }
        Ok(())
    }
}
