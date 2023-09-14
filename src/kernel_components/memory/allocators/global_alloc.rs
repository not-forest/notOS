/// Global Allocator for allocating DST's in kernel's memory heap.
///
/// The `GAllocator` structure serves as the central entity for this global allocator.
/// It uses an atomic counter to track the remaining memory in the arena and ensures
/// that memory allocations are properly aligned.
///
/// Deallocation is not supported, and using this allocator implies a fixed-size memory 
/// arena with no memory recycling. Therefore, this allocator is primarily suitable for
/// use cases where memory leaks can be managed and deallocation is not a requirement.

use core::alloc::{GlobalAlloc, Layout, Allocator};
use core::cell::UnsafeCell;
use core::ptr::{null_mut, NonNull};

use core::sync::atomic::{
    AtomicUsize,
    Ordering::SeqCst,
};

use super::BUMP_ALLOC;

/// Maximal size of the whole arena.
const ARENA_SIZE: usize = 128 * 1024;
/// Maximal size of the whole arena. Adjust the size as needed.
const MAX_ALIGN_SIZE: usize = 4096;

/// The main static global allocator's instance.
//#[global_allocator]
pub static GLOBAL_ALLOCATOR: GAllocator = GAllocator {
    arena: UnsafeCell::new([0x55; ARENA_SIZE]),
    remains: AtomicUsize::new(ARENA_SIZE),
};

/// A structure of global allocator for the OS.
#[repr(C, align(4096))]
pub struct GAllocator {
    arena: UnsafeCell<[u8; ARENA_SIZE]>,
    remains: AtomicUsize,
}

unsafe impl Sync for GAllocator {}

unsafe impl GlobalAlloc for GAllocator {
    /// Allocates memory with the specified layout in the global memory allocator.
    ///
    /// # Safety
    ///
    /// This function should only be used for creating statically-sized dynamic
    /// types. It may lead to memory leaks if not used carefully.
    ///
    /// # Returns
    ///
    /// Returns a pointer to the allocated memory block, or a null pointer if the
    /// requested alignment is not supported or if memory is exhausted.
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        match BUMP_ALLOC.allocate(layout) {
            Ok(address) => address.as_mut_ptr(),
            Err(alloc_error) => panic!("Allocation error: {alloc_error}. Memory overflow.")
        }
    }

    /// Does nothing. This function is needed to satisfy the GlobalAlloc trait bounds.
    ///
    /// # Safety
    ///
    /// This function is a no-op and doesn't deallocate memory.
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        BUMP_ALLOC.deallocate(
            NonNull::new(ptr).unwrap(),
            layout,
        )
    }
}






