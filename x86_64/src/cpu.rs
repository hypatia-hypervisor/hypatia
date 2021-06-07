// Copyright 2021  The Hypatia Authors
// All rights reserved
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

pub fn relax() {
    unsafe {
        core::arch::x86_64::_mm_pause();
    }
}
