#![feature(lang_items)]
#![cfg_attr(not(test), no_main)]
#![cfg_attr(not(test), no_std)]

#[cfg_attr(not(test), no_mangle)]
pub extern "C" fn main() {}

#[cfg(not(test))]
mod runtime {
    use core::panic::PanicInfo;

    #[panic_handler]
    pub extern "C" fn panic(_info: &PanicInfo) -> ! {
        #[allow(clippy::empty_loop)]
        loop {}
    }

    #[lang = "eh_personality"]
    extern "C" fn eh_personality() {}
}
