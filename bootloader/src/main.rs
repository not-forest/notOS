//! notOS Second Stage Bootloader.
//!
//! 
#![no_std]
#![no_main]
#![feature(used_with_arg)]
#![allow(non_snake_case)]

panic_custom::define_panic!(|info| loop {});

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    loop {}
}
