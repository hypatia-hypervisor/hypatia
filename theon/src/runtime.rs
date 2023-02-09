// Copyright 2021  The Hypatia Authors
// All rights reserved
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use alloc::alloc::Layout;
use core::panic::PanicInfo;

#[panic_handler]
pub extern "C" fn panic(info: &PanicInfo<'_>) -> ! {
    hypatia::panic::print_panic(info);
    #[allow(clippy::empty_loop)]
    loop {}
}

#[alloc_error_handler]
pub fn oom(layout: Layout) -> ! {
    panic!("Early allocation failed on size {}", layout.size());
}
