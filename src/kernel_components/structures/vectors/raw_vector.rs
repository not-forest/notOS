/// This is an abstraction over ptr and cap in vectors.

use core::{
    mem, 
    ptr::NonNull, 
    alloc::{Layout, GlobalAlloc},
};
use crate::kernel_components::memory::global_alloc::{GLOBAL_ALLOCATOR, GAllocator};

/// This node should be used in vectors and vector-like types.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RawVec<T: Sized> {
    /// Pointer to data on the heap.
    pub(crate) ptr: NonNull<T>,
    /// The overall capacity of the vector which is it's size.
    pub(crate) cap: usize,
}

impl<T> RawVec<T> {
    /// Creates a new instance of raw vector.
    pub(crate) fn new() -> Self {
        let cap = if mem::size_of::<T>() == 0 { usize::MAX } else { 0 };
        Self {
            ptr: NonNull::dangling(),
            cap,
        }
    }

    /// Inner function for growing the capacity and allocation memory on the heap
    /// 
    /// # Panics
    /// 
    /// Panic occur if the allocation is bigger than isize::MAX.
    pub(crate) fn grow(&mut self) {
        // since we set the capacity to usize::MAX when T has size 0,
        // getting to here necessarily means the Vec is overfull.
        assert!(mem::size_of::<T>() != 0, "Capacity overflow");

        let (new_cap, new_layout) = if self.cap == 0 {
            (1, Layout::array::<T>(1).unwrap())
        } else {
            let new_cap = 2 * self.cap;
            let new_layout = Layout::array::<T>(new_cap).unwrap();
            (new_cap, new_layout)
        };

        // Ensure that the new allocation doesn't exceed `isize::MAX` bytes.
        assert!(new_layout.size() <= isize::MAX as usize, "Allocation is too large.");

        let new_ptr = if self.cap == 0 {
            unsafe { GlobalAlloc::alloc(&GLOBAL_ALLOCATOR, new_layout) }
        } else {
            let old_layout = Layout::array::<T>(self.cap).unwrap();
            let old_ptr = self.ptr.as_ptr() as *mut u8;
            unsafe { GlobalAlloc::realloc(&GLOBAL_ALLOCATOR, old_ptr, old_layout, new_layout.size()) }
        };

        // If allocation fails, `new_ptr` will be null, in which case we abort.
        self.ptr = match NonNull::new(new_ptr as *mut T) {
            Some(p) => p,
            None => panic!("Allocation error!!!")
        };
        self.cap = new_cap;
    } 
}

// It is safe to share it through threads.
unsafe impl<T: Send> Send for RawVec<T> {}
unsafe impl<T: Sync> Sync for RawVec<T> {}

impl<T: Sized> Drop for RawVec<T> {
    fn drop(&mut self) {
        let elem_size = mem::size_of::<T>();

        if self.cap != 0 && elem_size != 0 {
            let layout = Layout::array::<T>(self.cap).unwrap();
            unsafe {
                GlobalAlloc::dealloc(&GLOBAL_ALLOCATOR, self.ptr.as_ptr() as *mut u8, layout);
            }
        }
    }
}