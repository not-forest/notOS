#![no_std]
#![no_main]
#![feature(custom_test_frameworks, used_with_arg)]
#![test_runner(crate::test_runner)]
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


pub mod kernel_components {
    pub mod vga_buffer;
}

use core::panic::PanicInfo;
use kernel_components::vga_buffer::Color;

#[no_mangle]
pub extern "C" fn _start() {

    #[cfg(test)]
    test_main();


    main();
}

fn main() -> ! {
    println!(Color::BLUE; "Hello there");

    loop {}
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    #[cfg(test)]
    println!(Color::RED; "[failed]");

    println!(Color::RED; "{}", info);
    loop {}
}


pub trait Testable {
    fn run(&self) -> ();
}

impl<T> Testable for T where T: Fn(), {
    fn run(&self) {
        print!("{}...    ", core::any::type_name::<T>());
        self();
        println!(Color::GREEN; "[ok]");
    }
}

#[no_mangle]
#[cfg(test)]
pub fn test_runner(tests: &[&dyn Fn()]) -> ! {
    println!(Color::LIGHTBLUE; "Running {} tests:", tests.len());
    for test in tests {
        test.run();
    }

    loop {}
}

#[test_case]
fn trivial_assertion() {
    assert_eq!(1, 1);
}

#[test_case]
fn trivial_assertion2() {
    assert_eq!(1, 2);
}