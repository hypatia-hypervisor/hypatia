// Copyright 2023  The Hypatia Authors
// All rights reserved
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

#[cfg(not(test))]
core::arch::global_asm!(include_str!("vmenter.S"), options(att_syntax));
