// Copyright 2021  The Hypatia Authors
// All rights reserved
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

#[derive(Clone, Copy, Debug)]
pub enum Type {
    Reserved,
    RAM,
    Loader,
    Module,
    ACPI,
    NonVolatile,
    Defective,
}

#[derive(Clone, Copy, Debug)]
pub struct Region {
    pub start: u64,
    pub end: u64,
    pub typ: Type,
}
