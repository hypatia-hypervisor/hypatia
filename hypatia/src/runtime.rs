// Copyright 2022 The Hypatia Authors
// All rights reserved
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

/// Generates the panic handler required for `no_std` binaries.
#[macro_export]
macro_rules! runtime {
    () => {
        #[cfg(all(target_os="none"))]
        mod no_std_runtime {
            use core::panic::PanicInfo;

            #[panic_handler]
            pub extern "C" fn panic(info: &PanicInfo) -> ! {
                hypatia::panic::print_panic(info);
                #[allow(clippy::empty_loop)]
                loop {}
            }
        }
    };
}
