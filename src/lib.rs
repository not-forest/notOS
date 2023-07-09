#![no_std]
#![no_main]

mod kernel_components {
    pub mod vga_buffer;
}

use core::panic::PanicInfo;

#[no_mangle]
#[allow(unreachable_code)]
pub extern "C" fn _start() -> ! {
    println!("Hello world! {}", "OS");

    loop {}
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {}
}
