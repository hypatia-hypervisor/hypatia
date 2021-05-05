#![feature(lang_items)]
#![cfg_attr(not(test), no_main)]
#![cfg_attr(not(test), no_std)]

use arch;
use arch::io::Sender;

#[cfg_attr(not(test), no_mangle)]
pub extern "C" fn main() {
    let mut port = arch::io::OutPort::new(0x3f8);
    port.send(b'a');
}

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
