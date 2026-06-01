//! Memory Management and Related Types.
//!
//! ## Optional
//!
//! This crate is optional and can be included to use heap memory at runtime.
//! Heapless implementation is used when this module is omitted.
#![no_std]
#![allow(non_snake_case)]

extern crate no_std_compat as std;

/// Page Definition.
///
/// Pages are equally sized chunks of memory. They are not unique to just 64-bit
/// architectures, therefore used in an abstraction model.
pub struct Page {
    /// Page number.
    pub num: usize,
} 
