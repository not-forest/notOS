#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]

mod kernel_components {
    pub mod vga_buffer;
}

use core::panic::PanicInfo;
use kernel_components::vga_buffer::Color;

#[no_mangle]
#[allow(unreachable_code)]
pub extern "C" fn _start() -> ! {
    println!(Color::BLUE; "Will this work? {}", "HMMMM");
    panic!("Some error");

    loop {}
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!(Color::RED; "{}", info);
    loop {}
}

#[cfg(test)]
pub fn test_runner(tests: &[&dyn Fn()]) {
    println!("Running {} tests:", tests.len());
    for test in tests {
        test();
    }
}
