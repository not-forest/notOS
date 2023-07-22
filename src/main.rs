#![no_std]
#![no_main]
#![feature(custom_test_frameworks, used_with_arg)]
#![test_runner(notOS::test_runner)]
#![reexport_test_harness_main = "test_main"]

#[link(name = "bootloader")]
extern "C" {
    fn initiate();
    fn header_start();
    fn header_end();
}

#[used]
static INITIATE_FUNC: unsafe extern "C" fn() = initiate;
#[used(linker)]
static HEADER_START_FUNC: unsafe extern "C" fn() = header_start;
#[used(linker)]
static HEADER_END_FUNC: unsafe extern "C" fn() = header_end;

use notOS::{println, print, kernel_components::vga_buffer::Color};

#[no_mangle]
pub extern "C" fn _start(_multiboot_information_address: usize) {
    #[cfg(test)]
    test_main();

    main();
}

#[allow(dead_code, unreachable_code)]
fn main() -> ! {
    println!(Color::BLUE; "Hello there");
    panic!("I would panic!");
    loop {}
}