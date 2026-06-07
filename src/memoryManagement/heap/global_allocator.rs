//! Global Allocator for allocating DST's in kernel's memory heap.
//!
//! Rust abstracts away heap management with it's [`GlobalAlloc`] trait and
//! `#[global_allocator]` procedural macro. This crate Defines a logical
//! block, which serves as a binding to other heap allocators. Depending
//! on the required algorithm, different allocators can be binded to a global
//! one, allowing to even switch them on runtime.

use std::alloc::{GlobalAlloc, Layout, Allocator};
use std::ptr::NonNull;
use std::sync::atomic::AtomicU8;

/// The main static global allocator's instance.
/// 
/// ## Warn
///
/// Global allocator is initialized with null pointer by default, therefore
/// it has no internal implementation binded. The [`GLOBAL_ALLOCATOR`] is
/// a single instance with static lifetime. Inner allocators however must be
/// created later and binded manually with no lifetime checks. 
#[global_allocator]
pub static mut GLOBAL_ALLOCATOR: _GlobalAllocator = _GlobalAllocator::new();

#[doc(hidden)]
pub struct _GlobalAllocator {
    inner: Option<*mut dyn Allocator>,
}

impl _GlobalAllocator {
    // Default global allocator initialization.
    #[doc(hidden)]
    const fn new() -> Self {
        Self {
            inner: None,
        }
    }

    #[allow(unsafe_op_in_unsafe_fn)]
    pub unsafe fn bind(&mut self, alloc: &mut dyn Allocator) {
        self.inner.replace(
            std::mem::transmute(
                std::ptr::from_mut(alloc)
            )
        );
    }
}

/* Global allocator instance implementation. */
unsafe impl GlobalAlloc for _GlobalAllocator {
    // Allocates memory using currently binded allocator.
    #[allow(unsafe_op_in_unsafe_fn)]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        match self.inner {
            Some(alloc) => 
                (*alloc).allocate(layout)
                .map(|ptr| ptr.as_mut_ptr())
                .unwrap_or(std::ptr::null_mut()),
            None => panic!("Tried to allocate heap object with no binded allocator implementation."),
        }
    }

    #[allow(unsafe_op_in_unsafe_fn)]
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        match self.inner {
            Some(alloc) => 
                (*alloc).deallocate(
                NonNull::new(ptr).expect("Tried to dereference a NULL pointer"),
                layout),
            None => panic!("Tried to deallocate heap object with no binded allocator implementation."),
        }
    }
}

/* Safe due to static lifetime and internal synchronization.*/
unsafe impl Sync for _GlobalAllocator {}
