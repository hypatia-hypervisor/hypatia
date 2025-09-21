// Copyright 2023  The Hypatia Authors
// All rights reserved
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

pub(crate) mod gdt;
pub(crate) mod idt;
pub(crate) mod tss;
mod xferv;

pub(crate) fn init() {
    idt::init();
    gdt::map();
    let tss = tss::init();
    gdt::init(tss);
}
