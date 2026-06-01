//! notOS Synchronization Primitives.
//!
//! Crate defines a set of synchronization primitives and their corresponding
//! types. It does not, however, defines any thread safe structures
#![no_std]
#![allow(non_snake_case)]

extern crate no_std_compat as std;

pub mod mutex;

pub use mutex::Mutex;
