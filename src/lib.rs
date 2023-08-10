// Main library space. It acts as a entry point of the crate.
// Every single outer files will be inside the kernel_components dir.
// Every single macro can be accessed within this crate. The main components will be also accessed from here.

#![no_std]
#![cfg_attr(test, no_main)]
#![allow(incomplete_features, unused, non_snake_case)]
#![feature(custom_test_frameworks, used_with_arg, error_in_core, ptr_metadata, generic_const_exprs)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]

// Main entry point for outer structures and objects
pub mod kernel_components {
    pub mod vga_buffer;
    pub mod error;

    pub mod structures {
        pub mod iternum;

        pub use iternum::IternumTrait;
    }

    pub mod instructions {
        pub mod random;
        pub mod interrupt;

        pub use random::{Random, RdRand, RdSeed};
    }

    pub mod sync {
        pub mod mutex;
        pub mod single;

        pub use mutex::{Mutex, MutexGuard};
        pub use single::{Once, Single};
    }

    pub mod memory {
        pub mod memory_module;
        pub mod tags;

        pub use memory_module::{InfoPointer, BootInfoHeader};
    }

    pub mod virtualization {
        pub mod process;
    }

}

use core::panic::PanicInfo;
pub use kernel_components::vga_buffer::Color;
pub use kernel_components::error::*;

// This function will be called on fatal errors in the system.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    #[cfg(test)]
    println!(Color::RED; "[failed]");
    println!(Color::RED; "{}", info);
    
    loop {}
}

// Custom trait for tests. It is only used when testing and do not affect overall performance.
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