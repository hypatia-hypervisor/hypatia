use std::env;

fn main() {
    let target = env::var("TARGET").unwrap();
    if target.as_str() == "x86_64-unknown-none-elf" {
        println!("cargo:rustc-link-arg-bins=--script=theon/src/x86_64/link.ld")
    }
}
