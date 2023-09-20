/// Leak allocator implementation
/// 
/// This allocator is suitable for kernels that do not prioritize DST too much,
/// and do not manipulate with them too much. The algorithm is the same as in the
/// Bump Allocator, except that it do not have any deallocation logic in it.
/// 
/// Use this allocator only when you are completely sure, that the DST's will always
/// live during the whole OS session or that the amount of those DST's wont cross the
/// size limit of allocator's arena.
/// 
/// # Scale
/// 
/// The size of heap arena is the same as the given at compile time. No growing methods
/// exists to make it bigger or shrink it. Do additional stack space or inner heap space
/// is used at runtime.
 
use crate::single;
use super::SubAllocator;
use core::alloc::{Allocator, Layout, GlobalAlloc, AllocError};
use core::ptr::NonNull;
use core::sync::atomic::{AtomicUsize, Ordering};

/// Start address of the memory heap. Use any address as long as it is not used.
pub const LEAK_ALLOC_HEAP_START: usize = 0o_000_001_000_000_0000;
/// Maximal size of the whole arena. Adjust the size as needed.
pub const LEAK_ALLOC_HEAP_ARENA: usize = 128 * 1024;

/// Static default allocator instance
single! {
    pub LEAK_ALLOC: LeakAlloc = LeakAlloc::new(
        LEAK_ALLOC_HEAP_START,
        LEAK_ALLOC_HEAP_START + LEAK_ALLOC_HEAP_ARENA,
    );
}

/// Implementation of leaking allocator.
/// 
/// A simple memory allocation algorithm used in scenarios where you want to allocate memory 
/// sequentially without any deallocation or reallocation.
/// 
/// # Initialization
/// 
/// Initializes a pointer to the start and the end of pre-allocated heap memory region.
/// 
/// # Allocation
/// 
/// When a memory allocation request comes in returns the pointer to the next area to allocate.
/// 
/// # Deallocation
/// 
/// No deallocation is implemented for this type of allocator. It will leak memory.
/// 
/// # Fragmentation
/// 
/// This allocator will always leak memory, therefore external fragmentation will grow at full
/// maximum.
/// 
/// # Thread safety
/// 
/// Allocation is thread safe, and uses lock-free algorithm to deal with memory regions.
#[derive(Debug)]
pub struct LeakAlloc {
    // The start of the heap
    start_ptr: NonNull<u8>,
    // The end of the heap
    end_ptr: NonNull<u8>,
    // Pointer to the next object. It must be an atomic, to create a lock-free allocations.
    next_ptr: AtomicUsize,
}

impl LeakAlloc {
    /// Creates a new leaking allocator instance.
    pub fn new(heap_start: usize, heap_end: usize) -> Self {
        Self {
            start_ptr: NonNull::new(heap_start as *mut u8).unwrap(),
            end_ptr: NonNull::new(heap_end as *mut u8).unwrap(),
            next_ptr: AtomicUsize::new(heap_start),
        }
    }

    /// Returns the address of the first ptr as usize
    pub fn start_ptr_addr(&self) -> usize {
        self.start_ptr.as_ptr() as usize
    }
    
    /// Returns the address of the last ptr as usize
    pub fn end_ptr_addr(&self) -> usize {
        self.end_ptr.as_ptr() as usize
    }
}

unsafe impl Allocator for LeakAlloc {
    /// Allocates memory for DST while leaking memory.
    /// 
    /// # Thread safety
    /// 
    /// ## This allocation algorithm is lock-free:
    /// 
    /// Every thread that wants to allocate some memory will enter a loop, in which they
    /// will get a pointer to next available memory address. All what they should do is just
    /// change this ptr to the next address for the next allocation to occur, and if they
    /// manage to do so, they will get the memory region.
    /// 
    /// Thread might loose it's chance to obtain address, if and only if, another thread obtained
    /// it faster which caused the CAS operation to fail.
    /// 
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        // Calculate a mask to enforce the required alignment.
        let align_mask = !(layout.align() - 1);

        loop {
            // Loads the current state of next ptr.
            let current_next_ptr = self.next_ptr.load(Ordering::Relaxed);
            let start_alloc = current_next_ptr & align_mask;
            let mut end_alloc = start_alloc.saturating_add(layout.size());

            if end_alloc <= self.end_ptr_addr() {
                #[cfg(debug_assertions)] {
                    crate::println!("Allocating {} bytes at {:#x}", layout.size(), current_next_ptr);
                }
                if let Ok(cas_current_next) = self.next_ptr.compare_exchange(
                    current_next_ptr,
                    end_alloc,
                    Ordering::SeqCst,
                    Ordering::Relaxed,
                ) {
                    return Ok(NonNull::slice_from_raw_parts(
                        NonNull::new(cas_current_next as *mut u8).unwrap(),
                        layout.size(),
                    ));
                }
            } else {
                return Err(AllocError)
            }
        }
    }

    /// Leak allocator cannot deallocate anything.
    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        // Leak memory
    }
}

impl SubAllocator for LeakAlloc {
    fn arena_size(&self) -> usize {
        LEAK_ALLOC_HEAP_ARENA
    }

    fn heap_addr(&self) -> usize {
        LEAK_ALLOC_HEAP_START
    }
}
