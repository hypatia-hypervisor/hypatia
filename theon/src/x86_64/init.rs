use crate::x86_64::multiboot1;

pub fn start(mbinfo_phys: u64) {
    uart::panic_println!("\nBooting Hypatia...");
    multiboot1::init(mbinfo_phys);
}
