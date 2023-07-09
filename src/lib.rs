#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]

mod kernel_components {
    pub mod vga_buffer;
}

use core::panic::PanicInfo;
use kernel_components::vga_buffer::Color;

#[no_mangle]
#[allow(unreachable_code)]
pub extern "C" fn _start() -> ! {
    println!("I can write a lot.");
    println!("A lot of {}", "colorful");
    println!(Color::MAGENTA; "Trully colorful");
    println!(Color::GREEN; Color::DARKGRAY; "For sure");
    println!(Color::BLACK; Color::WHITE; "Hello world! {}", "The world is in colors!");
    panic!("Some error oh no!");

    loop {}
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!(Color::RED; "{}", info);
    loop {}
}
