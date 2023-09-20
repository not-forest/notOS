/// Implementation of a bump allocator.
/// 
/// Whenever we allocate an object, we do a quick test that we have enough capacity left 
/// in the chunk, and then, assuming we have enough room, we move the bump pointer over by 
/// the size of object bytes and return the pointer to the space we just reserved for the 
/// object within the chunk.
/// 
/// # Scale
/// 
/// The size of heap arena is the same as the given at compile time. No growing methods
/// exists to make it bigger or shrink it. Do additional stack space or inner heap space
/// is used at runtime. Holes that are left without deallocation will be a result of external 
/// fragmentation.
use crate::single;
use super::SubAllocator;
use core::alloc::{Allocator, Layout, GlobalAlloc, AllocError};
use core::mem;
use core::ptr::{NonNull, self};
use core::sync::atomic::{AtomicUsize, Ordering};

/// Start address of the memory heap. Use any address as long as it is not used.
pub const BUMP_ALLOC_HEAP_START: usize = 0o_000_001_000_000_0000;
/// Maximal size of the whole arena. Adjust the size as needed.
pub const BUMP_ALLOC_HEAP_ARENA: usize = 128 * 1024;

/// Static default allocator instance
single! {
    pub BUMP_ALLOC: BumpAlloc = BumpAlloc::new(
        BUMP_ALLOC_HEAP_START,
        BUMP_ALLOC_HEAP_START + BUMP_ALLOC_HEAP_ARENA,
    );
}

/// Implementation of bump allocator.
/// 
/// A simple memory allocation algorithm used in scenarios where you want to allocate memory 
/// sequentially without the need for complex bookkeeping. It's often used in situations where 
/// memory fragmentation is not a concern.
/// 
/// # Initialization
/// 
/// Initializes a pointer to the start and the end of pre-allocated heap memory region.
/// 
/// # Allocation
/// 
/// When a memory allocation request comes in, checks if there is enough space in the memory 
/// region to fulfill the request. It allocates new objects by going forward through every object.
/// 
/// # Deallocation
/// 
/// Bump allocator deallocates objects by going backwards to the place where object is freed. Then
/// the pointer moves to that place.
/// 
/// # Fragmentation
/// 
/// Bump allocator do not handle memory fragmentation well. Once you allocate memory and move through
/// nodes, previously allocated memory cannot be reclaimed until the deallocation or reallocation.
/// 
/// # Thread safety
/// 
/// Bump allocators are typically not thread-safe. This one however uses a lock-free algorithms when allocating
/// and deallocating memory. More info in allocate(), deallocate() methods.
#[derive(Debug)]
pub struct BumpAlloc {
    // The start of the heap
    start_ptr: NonNull<u8>,
    // The end of the heap
    end_ptr: NonNull<u8>,
    // Temporary ptr for hole-checking.
    temp_ptr: AtomicUsize,
    // Pointer to the next object. It works in both directions (alloc and dealloc.). It must be
    // an atomic, to create a lock-free allocations and deallocations.
    next_ptr: AtomicUsize,
}

unsafe impl Sync for BumpAlloc {}

impl BumpAlloc {
    /// Creates a new bump allocator instance.
    pub fn new(heap_start: usize, heap_end: usize) -> Self {
        Self {
            start_ptr: NonNull::new(heap_start as *mut u8).unwrap(),
            end_ptr: NonNull::new(heap_end as *mut u8).unwrap(),
            temp_ptr: AtomicUsize::new(heap_start),
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

unsafe impl Allocator for BumpAlloc {
    /// Allocates memory for DST using bump algorithms.
    /// 
    /// # Thread safety
    /// 
    /// ## This allocation algorithm is lock-free:
    /// 
    /// Every thread that wants to allocate some memory will enter a loop, in which they
    /// will get a pointer to next available memory address, check for available size in the heap,
    /// and use CAS operation to obtain this memory address if it is still available.
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
            let current_temp_ptr = self.temp_ptr.load(Ordering::Relaxed);
            let start_alloc = current_next_ptr & align_mask;
            let mut end_alloc = start_alloc.saturating_add(layout.size());

            if end_alloc <= self.end_ptr_addr() {
                // If hole exist in given memory address, trying to write the data to the hole. If not, change the
                // pointer to the previous location, and continue the cycle.
                if current_temp_ptr > current_next_ptr {
                    if let Some(hole) = BumpHole::get_hole(current_next_ptr as *mut BumpHole) {
                        // If hole is enough for allocation
                        if !hole.is_enough_for(layout.size()) {
                            self.next_ptr.compare_exchange(
                                current_next_ptr,
                                hole.ptr,
                                Ordering::SeqCst,
                                Ordering::Relaxed,
                            );
                        } else {
                            end_alloc = hole.ptr;
                        }
                        
                        hole.delete();
                    }

                    if let Err(cas_old_temp) = self.temp_ptr.compare_exchange(
                        current_temp_ptr,
                        current_next_ptr,
                        Ordering::SeqCst,
                        Ordering::Relaxed,
                    ) {
                        // CAS operation failed, which means that the other thread did something with
                        // temp_ptr first. This thread must retry.
                        continue
                    }
                }

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

    /// Deallocates the DST.
    /// 
    /// Bump allocator do not use any data structure to store it's allocated ranges,
    /// so deallocation must be made forcefully. This means that we just move the pointer
    /// to the memory address where this deallocation occur, and the next allocation can 
    /// happen in this place, if it is big enough. If it is not big enough, the pointer will 
    /// move to previous location.
    /// 
    /// # Thread safety
    /// 
    /// ## This deallocation algorithm is lock-free:
    /// 
    /// Every thread that tries to deallocate, own the deallocation area. Therefore it can
    /// place a 'BumpHole' marker to that area. Afterwards it will try to change temp_ptr to
    /// current next_ptr, and then change next_ptr to starting area of deallocation (Since
    /// deallocation area is already aligned, it is possible via CAS). If it fails, that mean
    /// someone else did allocation or deallocation successfully. If it managed to change both 
    /// pointers, it means that deallocation is complete. The next time someone will allocate
    /// an object, they will first check if it is possible to fit that hole.
    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        // Check is the hole struct will fit in deallocated place.
        if mem::size_of::<BumpHole>() > layout.size() {
            #[cfg(debug_assertions)] {
                crate::println!("Ignoring the deallocation, because size is too small: {}", layout.size());
            }
            return
        }

        // Memory address of deallocation area.
        let start_dealloc = (ptr.as_ptr() as usize);

        // Setting the memory hole struct into the hole.
        BumpHole::set_hole(
            start_dealloc, 
            self.next_ptr.load(Ordering::Relaxed),
            layout.size()
        );

        loop {
            // Loads the current state of next node.
            let current_next_ptr = self.next_ptr.load(Ordering::Relaxed);
            let current_temp_ptr = self.temp_ptr.load(Ordering::Relaxed);

            if let Err(cas_old_temp) = self.temp_ptr.compare_exchange(
                current_temp_ptr,
                current_next_ptr,
                Ordering::SeqCst,
                Ordering::Relaxed,
            ) {
                // CAS operation failed, which means that the other thread did deallocation
                // first and changed the temp_ptr. This thread must retry again. 
                continue
            }

            if let Err(cas_old_next) = self.next_ptr.compare_exchange(
                current_next_ptr,
                start_dealloc,
                Ordering::SeqCst,
                Ordering::Relaxed,
            ) {
                // CAS operation failed, which means that the other thread did deallocation
                // first and changed the next_ptr. This thread must retry again. 
                continue
            }
            #[cfg(debug_assertions)] {
                crate::println!("Deallocating {} bytes from {:#x}", layout.size(), start_dealloc);
            }

            break
        }
    }
}

impl SubAllocator for BumpAlloc {
    fn arena_size(&self) -> usize {
        BUMP_ALLOC_HEAP_ARENA
    }

    fn heap_addr(&self) -> usize {
        BUMP_ALLOC_HEAP_START
    }
}

/// Hole which appears after deallocation of memory. It will be located when the previous object
/// was, and will only have two values inside: size of memory hole and a pointer to the previous location.
/// 
/// If the new allocated memory is not enough to fir this hole, bump allocator will try to fit it in the
/// location of this pointer. 
#[repr(C)]
struct BumpHole {
    size: usize,
    ptr: usize,
}

impl BumpHole {
    /// Marks the empty place with a hole
    fn set_hole(hole_ptr: usize, prev_ptr: usize, hole_size: usize) {
        let hole = BumpHole { size: hole_size, ptr: prev_ptr };

        unsafe {
            ptr::write_unaligned(hole_ptr as *mut BumpHole, hole);
        }
    }

    /// Deletes the hole from it's memory location.
    fn delete(&mut self) {
        unsafe {
            ptr::drop_in_place(self as *mut BumpHole);
        }
    }

    /// Gets the hole from the provided address, if exist.
    fn get_hole<'a>(hole_ptr: *mut BumpHole) -> Option<&'a mut Self> {
        unsafe { hole_ptr.as_mut() }
    }

    /// Checks if the hole has enough room for some item.
    fn is_enough_for(&self, alloc_size: usize) -> bool {
        self.size >= alloc_size
    }
}