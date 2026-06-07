//! Main kernel entry point.
//!
//! Defines common interface to connect lower-level bootloader code with kernel's
//! abstraction structures that configures and executes the kernel runtime.
#![no_std]
#![no_main]

panic_custom::define_panic!(|info| loop {});
