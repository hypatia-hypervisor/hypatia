#![feature(asm)]
#![feature(const_fn)]
#![feature(core_intrinsics)]
#![feature(global_asm)]
#![feature(lang_items)]
#![feature(naked_functions)]
#![feature(proc_macro_hygiene)]
#![cfg_attr(test, allow(dead_code))]
#![cfg_attr(not(test), no_std)]

pub mod panic;
mod x86_64;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}

pub mod arch {
    pub use crate::x86_64::*;
}
