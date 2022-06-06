// Copyright 2021  The Hypatia Authors
// All rights reserved
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use core::cmp;

#[allow(clippy::upper_case_acronyms)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Type {
    Reserved,
    RAM,
    Loader,
    Module,
    ACPI,
    NonVolatile,
    Defective,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Region {
    pub start: u64,
    pub end: u64,
    pub typ: Type,
}

impl Region {
    pub fn cmp(a: &Region, b: &Region) -> cmp::Ordering {
        match b.start.cmp(&a.start) {
            cmp::Ordering::Equal => b.end.cmp(&a.end),
            ordering => ordering,
        }
    }
}
