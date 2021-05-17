//! # Panic Utility Functions
//!
//! This module includes some utility functions useful for implementing panics in tasks.

use core::panic::PanicInfo;

/// Print a `PanicInfo` struct out to the console.
pub fn print_panic(info: &PanicInfo) {
    uart::panic_println!("\nPANIC: ");
    uart::panic_println!("*************** [ Cut Here ] *************");
    uart::panic_println!("{:#?}", info);
    uart::panic_println!("******************************************");
    uart::panic_println!("System halted.");
}
