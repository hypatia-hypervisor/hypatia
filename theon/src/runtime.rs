// Copyright 2021  The Hypatia Authors
// All rights reserved
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use core::panic::PanicInfo;

#[panic_handler]
pub extern "C" fn panic(info: &PanicInfo<'_>) -> ! {
    hypatia::panic::print_panic(info);
    #[allow(clippy::empty_loop)]
    loop {}
}
