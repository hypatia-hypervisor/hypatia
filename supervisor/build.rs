use std::env;

fn main() {
    let target = env::var("TARGET").unwrap();
    match target.as_str() {
        "x86_64-unknown-none-elf" => {
            println!("cargo:rustc-link-arg-bins=-Tsupervisor/src/link.ld")
        }
        _ => {}
    }
}