// Copyright 2021  The Hypatia Authors
// All rights reserved
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//!
//! Support for x86 segmentation.  In the host, we only support
//! 64-bit segmentation.
//!

use crate::{gdt, trap, StackIndex, CPL};

use bit_field::BitField;
use bitstruct::bitstruct;

bitstruct! {
    /// The Descriptor refers to a segment descriptor.
    #[derive(Clone, Copy, Debug)]
    pub struct Descriptor(u64) {
        reserved0: u32 = 0..32;
        reserved1: u8 = 32..40;
        pub accessed: bool = 40;
        pub readable: bool = 41;
        pub conforming: bool = 42;
        code: bool = 43;
        system: bool = 44;
        raw_privilege_level: u8 = 45..47;
        pub present: bool = 47;
        reserved2: u8 = 48..52;
        available: bool = 52;
        long: bool = 53;
        default32: bool = 54;
        granularity: bool = 55;
        reserved3: u8 = 56..64;
    }
}

impl Descriptor {
    /// Returns an empty descriptor.
    pub const fn empty() -> Descriptor {
        Descriptor(0)
    }

    /// Returns a null (0) descriptor.
    pub const fn null() -> Descriptor {
        Self::empty()
    }

    /// Returns a descriptor describing a 64-bit code segment.
    pub fn code64() -> Descriptor {
        const SYSTEM_MUST_BE_ONE: u64 = 1 << (32 + 12);
        Descriptor(SYSTEM_MUST_BE_ONE)
            .with_code(true)
            .with_present(true)
            .with_conforming(true)
            .with_accessed(true)
            .with_long(true)
            .with_privilege_level(CPL::Ring0)
    }

    #[must_use]
    pub fn with_privilege_level(self, level: CPL) -> Descriptor {
        self.with_raw_privilege_level(level as u8)
    }

    pub fn privilege_level(&self) -> CPL {
        CPL::try_from(self.raw_privilege_level()).expect("representable raw value")
    }
}

bitstruct! {
    /// The TaskStateDescriptor provides the hardware with sufficient
    /// information for the hardware to locate the TSS in memory.  The
    /// TSS, in turn, mostly holds stack pointers.
    #[derive(Clone, Copy, Debug)]
    pub struct TaskStateDescriptor(u128) {
        pub limit0: u16 = 0..16;
        pub base0: u16 = 16..32;
        pub base16: u8 = 32..40;
        mbo0: bool = 40;
        pub busy: bool = 41;
        mbz0: bool = 42;
        mbo1: bool = 43;
        mbz1: bool = 44;
        raw_privilege_level: u8 = 45..47;
        pub present: bool = 47;
        pub limit16: u8 = 48..52;
        pub avl: bool = 52;
        mbz2: bool = 53;
        mbz3: bool = 54;
        pub granularity: bool = 55;
        pub base24: u8 = 56..64;
        pub base32: u32 = 64..96;
        reserved0: u8 = 96..104;
        mbz4: u8 = 104..108;
        reserved1: u32 = 108..128;
    }
}

impl TaskStateDescriptor {
    /// Returns an empty TSS Descriptor.
    pub const fn empty() -> TaskStateDescriptor {
        const TYPE_TASK_AVAILABLE: u128 = 0b1001 << (8 + 32);
        TaskStateDescriptor(TYPE_TASK_AVAILABLE)
    }

    #[must_use]
    pub fn with_privilege_level(self, level: CPL) -> TaskStateDescriptor {
        self.with_raw_privilege_level(level as u8)
    }

    pub fn privilege_level(&self) -> CPL {
        CPL::try_from(self.raw_privilege_level()).expect("representable raw value")
    }
}

bitstruct! {
    /// Interrupt gate descriptors are entries in the IDT.
    #[derive(Clone, Copy, Default)]
    pub struct InterruptGateDescriptor(u128) {
        pub offset0: u16 = 0..16;
        pub segment_selector: u16 = 16..32;
        pub raw_stack_table_index: u8 = 32..35;
        mbz0: bool = 35;
        mbz1: bool = 36;
        mbz2: u8 = 37..40;
        fixed_type: u8 = 40..44;
        mbz3: bool = 44;
        raw_privilege_level: u8 = 45..47;
        pub present: bool = 47;
        pub offset16: u16 = 48..64;
        pub offset32: u32 = 64..96;
        pub reserved0: u32 = 96..128;
    }
}

impl InterruptGateDescriptor {
    pub const fn empty() -> InterruptGateDescriptor {
        const TYPE_INTERRUPT_GATE: u128 = 0b1110 << (32 + 8);
        InterruptGateDescriptor(TYPE_INTERRUPT_GATE)
    }

    pub fn new(thunk: &trap::Stub, stack_index: StackIndex) -> InterruptGateDescriptor {
        let ptr: *const trap::Stub = thunk;
        let va = ptr.addr();
        InterruptGateDescriptor::empty()
            .with_offset0(va.get_bits(0..16) as u16)
            .with_offset16(va.get_bits(16..32) as u16)
            .with_offset32(va.get_bits(32..64) as u32)
            .with_raw_stack_table_index(stack_index as u8)
            .with_segment_selector(gdt::GDT::code_selector())
            .with_present(true)
            .with_privilege_level(CPL::Ring0)
    }

    #[must_use]
    pub fn with_privilege_level(self, level: CPL) -> InterruptGateDescriptor {
        self.with_raw_privilege_level(level as u8)
    }

    pub fn privilege_level(&self) -> CPL {
        CPL::try_from(self.raw_privilege_level()).expect("representable raw value")
    }
}
