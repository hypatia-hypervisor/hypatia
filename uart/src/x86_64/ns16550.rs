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

    fn tx_ready(&mut self) -> bool {
        let mut lsr = self.lsr();
        let b = lsr.recv();
        b.get_bit(5)
    }

    pub fn putb(&mut self, b: u8) {
        while !self.tx_ready() {
            arch::cpu::pause();
        }
        self.thr().send(b);
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
