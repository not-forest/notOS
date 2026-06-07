//! Memory Management and Related Types.
//!
//! ## Optional
//!
//! This crate is optional and can be included to use heap memory at runtime.
//! Heapless implementation is used when this module is omitted.

#[cfg(CONFIG_HAS_VIRTUAL_MEMORY)]
mod virtualm;
#[cfg(CONFIG_HAS_HEAP)]
mod heap;

/// Physical Memory Address.
///
/// Value of actual address in linear memory model.
pub struct PhysicalAddress(usize);

/// Page Definition.
///
/// Pages are equally sized chunks of memory. They are not unique to just 64-bit
/// architectures, therefore used in an abstraction model.
pub struct Page {
    /// Page number.
    pub num: usize,
} 
