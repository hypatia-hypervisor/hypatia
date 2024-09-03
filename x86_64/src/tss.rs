// Copyright 2021  The Hypatia Authors
// All rights reserved
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! Task State Segment support.
//!
//! In the host, we only support the 64-bit TSS.

use crate::segment;
use crate::{HyperStack, StackIndex, CPL};

use bit_field::BitField;

/// Struct TSS represents the 64-bit task state structure,
/// as described in SDM Ch. 7.
///
/// Note that, instead of using a packed data structure and
/// u64 for things that are actually 64-bit quantities, we
/// use the 32-bit half words and the C ABI to guarantee
/// alignment and placement in the structure.

#[repr(C)]
pub struct TSS {
    _reserved0: u32,
    rsp0_lower: u32,
    rsp0_upper: u32,
    _rsp1_lower: u32,
    _rsp1_upper: u32,
    _rsp2_lower: u32,
    _rsp2_upper: u32,
    _reserved1: u32,
    _reserved2: u32,
    ist1_lower: u32,
    ist1_upper: u32,
    ist2_lower: u32,
    ist2_upper: u32,
    ist3_lower: u32,
    ist3_upper: u32,
    ist4_lower: u32,
    ist4_upper: u32,
    ist5_lower: u32,
    ist5_upper: u32,
    ist6_lower: u32,
    ist6_upper: u32,
    ist7_lower: u32,
    ist7_upper: u32,
    _reserved3: u32,
    _reserved4: u32,
    _reserved5: u16,
    io_map_base: u16,
}

impl TSS {
    /// Creates an empty TSS.
    pub const fn empty() -> TSS {
        TSS {
            _reserved0: 0,
            rsp0_lower: 0,
            rsp0_upper: 0,
            _rsp1_lower: 0,
            _rsp1_upper: 0,
            _rsp2_lower: 0,
            _rsp2_upper: 0,
            _reserved1: 0,
            _reserved2: 0,
            ist1_lower: 0,
            ist1_upper: 0,
            ist2_lower: 0,
            ist2_upper: 0,
            ist3_lower: 0,
            ist3_upper: 0,
            ist4_lower: 0,
            ist4_upper: 0,
            ist5_lower: 0,
            ist5_upper: 0,
            ist6_lower: 0,
            ist6_upper: 0,
            ist7_lower: 0,
            ist7_upper: 0,
            _reserved3: 0,
            _reserved4: 0,
            _reserved5: 0,
            io_map_base: core::mem::size_of::<TSS>() as u16,
        }
    }

    #[allow(clippy::needless_pass_by_ref_mut)]
    pub fn set_stack(&mut self, index: StackIndex, stack: &mut HyperStack) {
        let va = stack.top().addr() as u64;
        let lower = va.get_bits(0..32) as u32;
        let upper = va.get_bits(32..64) as u32;
        match index {
            StackIndex::Rsp0 => {
                self.rsp0_lower = lower;
                self.rsp0_upper = upper;
            }
            StackIndex::Ist1 => {
                self.ist1_lower = lower;
                self.ist1_upper = upper;
            }
            StackIndex::Ist2 => {
                self.ist2_lower = lower;
                self.ist2_upper = upper;
            }
            StackIndex::Ist3 => {
                self.ist3_lower = lower;
                self.ist3_upper = upper;
            }
            StackIndex::Ist4 => {
                self.ist4_lower = lower;
                self.ist4_upper = upper;
            }
            StackIndex::Ist5 => {
                self.ist5_lower = lower;
                self.ist5_upper = upper;
            }
            StackIndex::Ist6 => {
                self.ist6_lower = lower;
                self.ist6_upper = upper;
            }
            StackIndex::Ist7 => {
                self.ist7_lower = lower;
                self.ist7_upper = upper;
            }
        }
    }

    /// Returns a fully-formed TSS descriptor for this TSS.
    pub fn descriptor(&self) -> segment::TaskStateDescriptor {
        let ptr: *const Self = self;
        let va = ptr.addr() as u64;
        segment::TaskStateDescriptor::empty()
            .with_limit0(core::mem::size_of::<TSS>() as u16 - 1)
            .with_base0(va.get_bits(0..16) as u16)
            .with_base16(va.get_bits(16..24) as u8)
            .with_privilege_level(CPL::Ring0)
            .with_present(true)
            .with_avl(true)
            .with_granularity(true)
            .with_base24(va.get_bits(24..32) as u8)
            .with_base32(va.get_bits(32..64) as u32)
    }
}
