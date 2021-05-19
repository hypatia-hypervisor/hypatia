#![feature(asm)]
#![feature(core_intrinsics)]
#![feature(global_asm)]
#![feature(lang_items)]
#![feature(naked_functions)]
#![feature(proc_macro_hygiene)]
#![cfg_attr(test, allow(dead_code))]
#![cfg_attr(not(test), no_main)]
#![cfg_attr(not(test), no_std)]

mod x86_64;

#[cfg_attr(not(test), no_mangle)]
pub extern "C" fn main() -> ! {
    x86_64::init::start();
    panic!("main: end reached");
}

#[no_mangle]
pub extern "C" fn apmain() -> ! {
    panic!("apmain");
}

#[cfg(not(test))]
mod runtime {
    use core::panic::PanicInfo;

    #[panic_handler]
    pub extern "C" fn panic(info: &PanicInfo) -> ! {
        libhypatia::panic::print_panic(info);
        #[allow(clippy::empty_loop)]
        loop {}
    }

    #[lang = "eh_personality"]
    extern "C" fn eh_personality() {}
}
