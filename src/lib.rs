#![no_std]
#![cfg_attr(test, no_main)]
#![feature(custom_test_frameworks, used_with_arg, error_in_core)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]

pub mod kernel_components {
    pub mod vga_buffer;
    pub mod error;

    pub mod instructions {
        pub mod interrupt;
    }

    pub mod sync {
        pub mod mutex;
        pub mod single;

        pub use mutex::{Mutex, MutexGuard};
        pub use single::{Once, Single};
    }

    pub mod memory {
        pub mod memory_module;
    }

}

use core::panic::PanicInfo;
use kernel_components::vga_buffer::Color;

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

pub fn test_runner(tests: &[&dyn Fn()]) -> ! {
    println!(Color::LIGHTBLUE; "Running {} tests:", tests.len());
    for test in tests {
        test.run();
    }

    loop {}
}