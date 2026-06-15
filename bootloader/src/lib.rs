//! Global bootloader module.
//!
//! Provides kernel with all necessary information to start allocating
//! hardware resources, while preserving hardware independence.
#![no_std]
#![allow(non_snake_case)]

extern crate no_std_compat as std;

/// Compile-time hardware-specific generated constants from `bootloader.yaml` file.
///
/// While kernel's constants are consistently reexported publicly, each architecture
/// may define specific configuration constants for their implementation.
#[allow(clippy::unreadable_literal)]
pub mod constants {
    build_const::build_const!(env!("CARGO_PKG_NAME"));
}
