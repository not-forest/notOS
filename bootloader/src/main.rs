//! notOS Second Stage Bootloader.
//!
//! 
#![no_std]
#![no_main]
#![allow(non_snake_case)]

panic_custom::define_panic!(|_| loop {});

#[unsafe(no_mangle)]
#[allow(clippy::empty_loop)]
pub extern "C" fn _start() -> ! {
    loop {}
}
