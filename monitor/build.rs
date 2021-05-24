use std::env;

fn main() {
    let target = env::var("TARGET").unwrap();
    if target.as_str() == "x86_64-unknown-none-elf" {
        println!("cargo:rustc-link-arg-bins=-Tmonitor/src/link.ld")
    }
}
