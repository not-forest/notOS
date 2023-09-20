/// Global Allocator for allocating DST's in kernel's memory heap.
///
/// The `GAllocator` structure serves as the central entity for kernel's heap
/// memory management. It uses the algorithm of inner allocator, that lies inside
/// the struct. By default, leaking allocator is being used, but it can be changed
/// at runtime by use() method.

use core::alloc::{GlobalAlloc, Layout, Allocator};
use core::cell::UnsafeCell;
use core::ptr::{null_mut, NonNull};

use super::*;
use crate::single;
use crate::kernel_components::structures::Single;
use core::sync::atomic::{
    AtomicUsize,
    Ordering::SeqCst,
};

/// The main static global allocator's instance.
/// 
/// # Default
/// 
/// By default, global allocator will use the leaking allocator. It is made this way,
/// because it should use something by default, to not generate errors, and also, this is
/// one of very few allocators, that will not fill stack or heap memory when initialized.
/// 
/// It is recommended to change the allocator, if it's behavior and algorithm is not
/// expected (usually leaking memory is not a good thing.).
/// 
/// # Important
/// 
/// Default values should be changed before some allocations will occur, otherwise
/// it will leak memory. 
//#[global_allocator]
single! {
    pub mut GLOBAL_ALLOCATOR: GAllocator = GAllocator {
        heap_addr: leak_alloc::LEAK_ALLOC_HEAP_START,
        arena_size: leak_alloc::LEAK_ALLOC_HEAP_ARENA,
        allocator: &*LEAK_ALLOC,
    };
}

/// A structure of global allocator for the OS.
#[repr(C, align(4096))]
pub struct GAllocator {
    pub heap_addr: usize,
    pub arena_size: usize,
    allocator: &'static dyn SubAllocator,
}

impl GAllocator {
    /// Set the corresponding allocator for use in the kernel.
    /// 
    /// ## Important
    /// 
    /// This function must be used before the kernel has been remapped.
    /// If the allocator will be changed after the remapping process, the
    /// area of previous allocator will not be used in any way possible, plus
    /// the new heap memory regions won't be mapped.
    pub fn r#use<A>(&mut self, allocator: &'static Single<A>) where
        A: SubAllocator + 'static
    {
        self.heap_addr = allocator.heap_addr();
        self.arena_size = allocator.arena_size();
        self.allocator = &**allocator
    }
}

unsafe impl Sync for GAllocator {}

unsafe impl GlobalAlloc for GAllocator {
    /// Allocates memory with the specified layout, using the inner allocator.
    /// 
    /// # Returns
    ///
    /// Returns a pointer to the allocated memory block, or panics, if the pointer is null.
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        match self.allocator.allocate(layout) {
            Ok(address) => address.as_mut_ptr(),
            Err(alloc_error) => panic!("Allocation error: {alloc_error}. Memory overflow.")
        }
    }

    /// This function calls the inner allocator's deallocate function.
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.allocator.deallocate(
            NonNull::new(ptr).unwrap(),
            layout,
        )
    }
}

/// Trait for sub allocators that work within the global allocator.
/// 
/// The 'GAlloc' is using one of such allocators as main algorithm to manipulate
/// with heap memory. It should come with 'Allocator' trait, and share some additional
/// data, that must be provided when implementing own allocators.
pub trait SubAllocator: Allocator {
    /// This method must return a valid address of the heap start.
    /// 
    /// # Safety
    /// 
    /// Invalid heap start address can lead to memory leaks and page faults.
    fn heap_addr(&self) -> usize;
    /// This method must return a valid size of heap arena for this allocator.
    /// 
    /// # Safety
    /// 
    /// Invalid heap start address can lead to memory leaks and page faults.
    fn arena_size(&self) -> usize;
}