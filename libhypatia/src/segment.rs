/// Call this macro from within each segment to include all the per-segment boilerplate.
#[macro_export]
macro_rules! define_segment {
    () => {
        mod _segment {
            use core::panic::PanicInfo;

            #[cfg(not(test))]
            #[panic_handler]
            pub extern "C" fn panic(_info: &PanicInfo) -> ! {
                #[allow(clippy::empty_loop)]
                loop {}
            }

            #[cfg(not(test))]
            #[lang = "eh_personality"]
            extern "C" fn eh_personality() {}

            #[cfg_attr(not(test), no_mangle)]
            pub extern "C" fn main() {
                crate::init();
            }
        }
    };
}
