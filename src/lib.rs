//! notOS Kernel Main Library Root.
//!
//!
#![no_std]
#![allow(non_snake_case)]

extern crate no_std_compat as std;

/// Compile-time hardware-specific generated constants from `core.yaml` file.
///
/// ## Constants
///
/// All constants are uppercase symbols starting with "CONFIG_" prefix and generated
/// at pre-build stage by parsing YAML file of the corresponding hardware.
pub mod constants {
    build_const::build_const!("core.yaml");
}

/// Structures and definitions for managing memory.
mod memoryManagement;
/// Kernel's primitive and compound data structures.
mod structures;
/// Synchronization primitives and data structures.
mod sync;
/// Task virtualization and scheduling.
mod taskVirtualization;
