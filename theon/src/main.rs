// Copyright 2021  The Hypatia Authors
// All rights reserved
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

#![feature(alloc_error_handler)]
#![feature(allocator_api)]
#![feature(asm)]
#![feature(core_intrinsics)]
#![feature(global_asm)]
#![feature(lang_items)]
#![feature(naked_functions)]
#![feature(proc_macro_hygiene)]
#![cfg_attr(test, allow(dead_code))]
#![cfg_attr(not(test), no_main)]
#![cfg_attr(not(test), no_std)]

extern crate alloc;

mod allocator;
mod x86_64;

#[cfg_attr(not(test), no_mangle)]
pub extern "C" fn main(mbinfo_phys: u64) -> ! {
    x86_64::init::start(mbinfo_phys);
    panic!("main: trapstubs = {:#x}", arch::trap::stubs as u64);
}

#[no_mangle]
pub extern "C" fn apmain() -> ! {
    panic!("apmain");
}

#[cfg(not(test))]
mod runtime {
    use alloc::alloc::Layout;
    use core::panic::PanicInfo;

    #[panic_handler]
    pub extern "C" fn panic(info: &PanicInfo) -> ! {
        libhypatia::panic::print_panic(info);
        #[allow(clippy::empty_loop)]
        loop {}
    }

    #[alloc_error_handler]
    pub fn oom(layout: Layout) -> ! {
        panic!("Early allocation failed on size {}", layout.size());
    }

    #[lang = "eh_personality"]
    extern "C" fn eh_personality() {}
}
