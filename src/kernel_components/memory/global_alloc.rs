/// Global Allocator for allocating Sub-Allocators or static DST's
///
/// The `GAllocator` structure serves as the central entity for this global allocator.
/// It uses an atomic counter to track the remaining memory in the arena and ensures
/// that memory allocations are properly aligned.
///
/// Deallocation is not supported, and using this allocator implies a fixed-size memory 
/// arena with no memory recycling. Therefore, this allocator is primarily suitable for
/// use cases where memory leaks can be managed and deallocation is not a requirement.

use core::alloc::{GlobalAlloc, Layout};
use core::cell::UnsafeCell;
use core::ptr::null_mut;

use core::sync::atomic::{
    AtomicUsize,
    Ordering::SeqCst,
};

/// Maximal size of the whole arena. Global allocator's arena will contain inner allocators within. 
const ARENA_SIZE: usize = 128 * 1024;
/// Maximal possible align size of a Layout.
const MAX_ALIGN_SIZE: usize = 4096;

/// The main static global allocator's instance.
#[global_allocator]
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
    /// types or sub-allocators. It may lead to memory leaks if not used carefully.
    ///
    /// # Returns
    ///
    /// Returns a pointer to the allocated memory block, or a null pointer if the
    /// requested alignment is not supported or if memory is exhausted.
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let size = layout.size(); // Get the size of the memory block to allocate.
        let align = layout.align(); // Get the required alignment of the memory block.
        let align_mask = !(align - 1); // Calculate a mask to enforce the required alignment.

        if align > MAX_ALIGN_SIZE {
            return null_mut() // If alignment is not supported, return null pointer.
        }

        let mut allocated = 0; // Initialize a variable to track the allocated size.

        // Proceed with the allocation
        if self.remains
            .fetch_update(SeqCst, SeqCst, |mut remains| {
                if size > remains {
                    return None; // If there's not enough memory left, return None.
                }

                remains -= size;
                remains &= align_mask; // Apply the alignment mask to the remaining size to floor the value.
                allocated = remains; 
                Some(allocated)
            }).is_err() {
                return null_mut() // If updating the remains fails, return null pointer.
            }

            // Get a pointer to the allocated memory and add the allocated size to it.
            self.arena.get().cast::<u8>().add(allocated)
    }

    /// Does nothing. This function is needed to satisfy the GlobalAlloc trait bounds.
    ///
    /// # Safety
    ///
    /// This function is a no-op and doesn't deallocate memory. It is marked as
    /// unsafe to emphasize that using this allocator implies memory leaks.
    unsafe fn dealloc(&self, _: *mut u8, _: Layout) {}
}






