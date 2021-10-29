// Copyright 2021  The Hypatia Authors
// All rights reserved
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use bitstruct::bitstruct;

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug)]
pub enum DeliveryMode {
    Fixed = 0b000,
    SMI = 0b010,
    NMI = 0b100,
    Init = 0b101,
    SIPI = 0b110,
}

#[derive(Debug)]
pub enum DestinationMode {
    Physical,
    Logical,
}

#[derive(Debug)]
pub enum Level {
    DeAssert,
    Assert,
}

#[derive(Debug)]
pub enum TriggerMode {
    Edge,
    Level,
}

#[derive(Debug)]
pub enum DestinationShorthand {
    Myself = 0b01,
    AllIncludingSelf = 0b10,
    AllButSelf = 0b11,
}

bitstruct! {
    #[derive(Clone, Copy)]
    pub struct ICR(pub u64) {
        vector: u8 = 0..8;
        raw_delivery_mode: u8 = 8..11;
        destination_mode: DestinationMode = 11;
        level: Level = 14;
        trigger_mode: TriggerMode = 15;
        raw_destination_shorthand: u8 = 18..20;
        destination: u32 = 32..64;
    }
}

impl ICR {
    pub fn with_delivery_mode(self, mode: DeliveryMode) -> ICR {
        self.with_raw_delivery_mode(mode as u8)
    }

    pub fn delivery_mode(self) -> Result<DeliveryMode, u8> {
        match self.raw_delivery_mode() {
            0b000 => Ok(DeliveryMode::Fixed),
            0b010 => Ok(DeliveryMode::SMI),
            0b100 => Ok(DeliveryMode::NMI),
            0b101 => Ok(DeliveryMode::Init),
            0b110 => Ok(DeliveryMode::SIPI),
            o => Err(o),
        }
    }

    pub fn with_destination_shorthand(self, shorthand: Option<DestinationShorthand>) -> ICR {
        self.with_raw_destination_shorthand(shorthand.map_or(0, |s| s as u8))
    }

    pub fn destination_shorthand(self) -> Option<DestinationShorthand> {
        match self.raw_destination_shorthand() {
            0b00 => None,
            0b01 => Some(DestinationShorthand::Myself),
            0b10 => Some(DestinationShorthand::AllIncludingSelf),
            0b11 => Some(DestinationShorthand::AllButSelf),
            _ => panic!("unrepresentable value in raw destination shorthand"),
        }
    }
}

impl bitstruct::FromRaw<bool, Level> for ICR {
    fn from_raw(raw: bool) -> Level {
        match raw {
            false => Level::DeAssert,
            true => Level::Assert,
        }
    }
}

impl bitstruct::IntoRaw<bool, Level> for ICR {
    fn into_raw(level: Level) -> bool {
        match level {
            Level::DeAssert => false,
            Level::Assert => true,
        }
    }
}

impl bitstruct::FromRaw<bool, DestinationMode> for ICR {
    fn from_raw(raw: bool) -> DestinationMode {
        match raw {
            false => DestinationMode::Physical,
            true => DestinationMode::Logical,
        }
    }
}

impl bitstruct::IntoRaw<bool, DestinationMode> for ICR {
    fn into_raw(mode: DestinationMode) -> bool {
        match mode {
            DestinationMode::Physical => false,
            DestinationMode::Logical => true,
        }
    }
}

impl bitstruct::FromRaw<bool, TriggerMode> for ICR {
    fn from_raw(raw: bool) -> TriggerMode {
        match raw {
            false => TriggerMode::Edge,
            true => TriggerMode::Level,
        }
    }
}

impl bitstruct::IntoRaw<bool, TriggerMode> for ICR {
    fn into_raw(mode: TriggerMode) -> bool {
        match mode {
            TriggerMode::Edge => false,
            TriggerMode::Level => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::assert_matches::assert_matches;

    #[test]
    fn fixed() {
        let icr =
            ICR(0).with_vector(0xFE).with_delivery_mode(DeliveryMode::Fixed).with_destination(2);
        assert_eq!(icr.0, 0x0000_0002_0000_00fe);
    }

    #[test]
    fn fixed_shorthand() {
        let icr = ICR(0)
            .with_vector(0xFE)
            .with_delivery_mode(DeliveryMode::Fixed)
            .with_destination_shorthand(Some(DestinationShorthand::AllButSelf));
        assert_eq!(icr.0, 0x0000_0000_000c_00fe);
    }

    #[test]
    fn sipi() {
        let icr = ICR(0)
            .with_vector(0x1F)
            .with_delivery_mode(DeliveryMode::SIPI)
            .with_destination_mode(DestinationMode::Logical)
            .with_level(Level::Assert)
            .with_destination_shorthand(None)
            .with_trigger_mode(TriggerMode::Edge)
            .with_destination_shorthand(None)
            .with_destination(0x1F);
        assert_eq!(0x0000_001f_0000_4e1f, icr.0);
    }

    #[test]
    fn parse_sipi() {
        let icr = ICR(0x0000_001f_0000_4e2f);
        assert_eq!(icr.vector(), 0x2F);
        assert_matches!(icr.delivery_mode(), Ok(DeliveryMode::SIPI));
        assert_matches!(icr.destination_mode(), DestinationMode::Logical);
        assert_matches!(icr.level(), Level::Assert);
        assert_matches!(icr.destination_shorthand(), None);
        assert_matches!(icr.trigger_mode(), TriggerMode::Edge);
        assert_matches!(icr.destination(), 0x1F);
    }

    #[test]
    fn parse_bad_delivery_mode() {
        let icr = ICR(0x0000_001f_0000_4f2f);
        assert_eq!(icr.vector(), 0x2F);
        assert_matches!(icr.delivery_mode(), Err(0b111));
        assert_matches!(icr.destination_mode(), DestinationMode::Logical);
        assert_matches!(icr.level(), Level::Assert);
        assert_matches!(icr.destination_shorthand(), None);
        assert_matches!(icr.trigger_mode(), TriggerMode::Edge);
        assert_matches!(icr.destination(), 0x1F);
    }

    #[test]
    fn parse_with_some_shorthand() {
        let icr = ICR(0x0000_0000_000c_4f2f);
        assert_eq!(icr.vector(), 0x2F);
        assert_matches!(icr.delivery_mode(), Err(0b111));
        assert_matches!(icr.destination_mode(), DestinationMode::Logical);
        assert_matches!(icr.level(), Level::Assert);
        assert_matches!(icr.destination_shorthand(), Some(DestinationShorthand::AllButSelf));
        assert_matches!(icr.trigger_mode(), TriggerMode::Edge);
        assert_matches!(icr.destination(), 0);
    }
}
