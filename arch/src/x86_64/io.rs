// Copyright 2021  The Hypatia Authors
// All rights reserved
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use core::marker::PhantomData;

pub trait PortSized {}
impl PortSized for u8 {}
impl PortSized for u16 {}
impl PortSized for u32 {}

pub trait PortMarker<T: PortSized> {
    fn addr(&mut self) -> u16;
}

pub trait Out<T: PortSized>: PortMarker<T> {}
pub trait In<T: PortSized>: PortMarker<T> {}

pub trait Receiver<T: PortSized> {
    fn recv(&mut self) -> T;
}

impl<T: PortMarker<u8> + In<u8>> Receiver<u8> for T {
    fn recv(&mut self) -> u8 {
        unsafe { x86::io::inb(self.addr()) }
    }
}

impl<T: PortMarker<u16> + In<u16>> Receiver<u16> for T {
    fn recv(&mut self) -> u16 {
        unsafe { x86::io::inw(self.addr()) }
    }
}

impl<T: PortMarker<u32> + In<u32>> Receiver<u32> for T {
    fn recv(&mut self) -> u32 {
        unsafe { x86::io::inl(self.addr()) }
    }
}

pub trait Sender<T: PortSized> {
    fn send(&mut self, datum: T);
}

impl<T: PortMarker<u8> + Out<u8>> Sender<u8> for T {
    fn send(&mut self, datum: u8) {
        unsafe {
            x86::io::outb(self.addr(), datum);
        }
    }
}

impl<T: PortMarker<u16> + Out<u16>> Sender<u16> for T {
    fn send(&mut self, datum: u16) {
        unsafe {
            x86::io::outw(self.addr(), datum);
        }
    }
}

impl<T: PortMarker<u32> + Out<u32>> Sender<u32> for T {
    fn send(&mut self, datum: u32) {
        unsafe {
            x86::io::outl(self.addr(), datum);
        }
    }
}

pub struct OutPort<T>(u16, PhantomData<T>);

impl<T> OutPort<T> {
    pub const fn new(addr: u16) -> OutPort<T> {
        OutPort(addr, PhantomData)
    }
}

impl<T: PortSized> PortMarker<T> for OutPort<T> {
    fn addr(&mut self) -> u16 {
        self.0
    }
}

impl<T: PortSized> Out<T> for OutPort<T> {}

pub struct InPort<T>(u16, PhantomData<T>);

impl<T> InPort<T> {
    pub const fn new(addr: u16) -> InPort<T> {
        InPort(addr, PhantomData)
    }
}

impl<T: PortSized> PortMarker<T> for InPort<T> {
    fn addr(&mut self) -> u16 {
        self.0
    }
}

impl<T: PortSized> In<T> for InPort<T> {}

pub struct Port<T>(u16, PhantomData<T>);

impl<T> Port<T> {
    pub const fn new(port: u16) -> Port<T> {
        Port(port, PhantomData)
    }
}

impl<T: PortSized> PortMarker<T> for Port<T> {
    fn addr(&mut self) -> u16 {
        self.0
    }
}

impl<T: PortSized> Out<T> for Port<T> {}
impl<T: PortSized> In<T> for Port<T> {}
