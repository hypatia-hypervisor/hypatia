#![cfg_attr(not(test), no_std)]

mod x86_64;

pub use x86_64::*;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
