//! Heap Allocator and Data Types.
//!
//! ## Optional
//!
//! This crate is optional and can be included to use heap memory at runtime.
//! Heapless implementation is used when this module is omitted.

extern crate no_std_compat as std;

mod global_allocator;

use crate::constants::CONFIG_PAGE_SIZE;

/// Heap Arena
///
/// Memory region defined by `start` address and `pages`, which is length 
/// in pages.
///
/// ## Note
///
/// Page size differs for different architecture builds. Consult the specific 
/// `config.yaml` file for used architecture for more details.
pub struct HeapArena {
    /// Start address of the arena.
    start: usize,
    /// Amount of pages the arena takes.
    pages: usize,
}

impl HeapArena {
    /// Creates a new instance of [`HeapArena`]
    #[inline(always)]
    pub const fn new(start: usize, pages: usize) -> Self {
        Self { start, pages }
    }

    /// Gets end address of the arena.
    #[inline(always)]
    pub const fn end(&self) -> usize {
        self.start + self.pages * CONFIG_PAGE_SIZE as usize
    }
}
