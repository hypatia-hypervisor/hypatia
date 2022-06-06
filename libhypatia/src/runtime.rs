// Copyright 2022 The Hypatia Authors
// All rights reserved
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

/// Code that is required in every independent no_std crate, whether it is a segment or a task.
#[macro_export]
macro_rules! __runtime_boilerplate {
    () => {
        #[cfg(not(test))]
        mod _no_std_boilerplate {
            use core::panic::PanicInfo;

            #[panic_handler]
            pub extern "C" fn panic(_info: &PanicInfo) -> ! {
                #[allow(clippy::empty_loop)]
                loop {}
            }
        }
    };
}

/// Call this macro once per segment to include all required boilerplate.
///
/// Takes the function name of the init routine for the segment.
#[macro_export]
macro_rules! define_segment {
    ($init:ident) => {
        libhypatia::__runtime_boilerplate!();
        mod _no_std_segment {
            // cfg_attr(not(test), no_mangle) here acts as a hack that acts as a rename for the
            // 'main' function when compiled as a test, thus getting out of the way of the test
            // compile's main.
            #[start]
            #[cfg_attr(not(test),no_mangle)]
            pub extern "C" fn main() {
                crate::$init()
            }
        }
    };
}

/// Call this macro once per task to include all required boilerplate.
#[macro_export]
macro_rules! define_task {
    () => {
        libhypatia::__runtime_boilerplate!();
    };
}
