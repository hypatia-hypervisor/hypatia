// Copyright 2021  The Hypatia Authors
// All rights reserved
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

#![feature(alloc_error_handler)]
#![feature(allocator_api)]
#![feature(lang_items)]
#![feature(naked_functions)]
#![feature(start)]
#![cfg_attr(not(test), no_main)]
#![cfg_attr(not(test), no_std)]

extern crate alloc;

mod allocator;
mod theon;
mod x86_64;

#[cfg_attr(not(test), start, no_mangle)]
pub extern "C" fn main(mbinfo_phys: u64) -> ! {
    let multiboot = x86_64::init::start(mbinfo_phys);
    let crate::x86_64::multiboot1::InitInfo { memory_regions, regions, modules } = multiboot.info();
    core::mem::drop(memory_regions);
    uart::panic_println!("regions: {:#x?}", regions);
    let bins = modules.iter().find(|m| m.name == Some("bin.a")).expect("found 'bin.a' in modules");
    let archive = goblin::archive::Archive::parse(bins.bytes).expect("cannot parse bin.a");
    uart::panic_println!("Binary archive: {:#x?}", archive);
    unsafe { core::arch::asm!("int3") };
    panic!("main: trapstubs = {:#x}", arch::trap::stubs as usize);
}

#[cfg_attr(test, allow(dead_code))]
#[no_mangle]
pub extern "C" fn apmain() -> ! {
    panic!("apmain");
}

#[cfg(not(test))]
mod runtime;
