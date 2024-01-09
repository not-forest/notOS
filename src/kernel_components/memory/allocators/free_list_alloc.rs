/// A free-list allocator implementation.
/// 
/// This allocator is using a list to check on used memory regions and provide
/// allocation and deallocation. It is suitable for most of the systems and can
/// be initiated with a special config needed for the system.
/// 
/// Use this allocator if you need a dynamic and flexible allocator that can satisfy
/// the allocation and deallocation in any order and size. External fragmentation will
/// occur when allocator's nodes will be divided in a small chunks of memory. It will cause
/// the grow of loosed memory. Almost each allocation will cause the division of the node,
/// this will lead to several nodes spawning. Therefore, fragmentation that occurs always
/// can be counted as (AMOUNT_OF_ALLOCATIONS - AMOUNT OF DEALLOCATIONS) * 16, where 16 is 
/// a size of node structure.
/// 
/// ## Scale
/// 
/// The size of heap arena is the same as the given at compile time. No growing methods
/// exists to make it bigger or shrink it. The allocator uses it's own small heap to store
/// the free list data.
/// 
/// ## Search
/// 
/// The allocator can search through the list in a various ways. This can be configured out
/// by the allocator's methods. Available search strategies are:
/// 
/// # First Fit:
/// 
/// This search strategy looks for the first available memory block in the linked list that 
/// is large enough to satisfy the allocation request. It stops searching once it finds a 
/// suitable block. This strategy is simple but may lead to memory fragmentation.
/// 
/// # Best Fit:
/// 
/// The best-fit strategy searches for the smallest available memory block that can accommodate
/// the allocation request. It aims to minimize fragmentation but may require more time to 
/// search for the best-fitting block.
/// 
/// # Worst Fit:
/// 
/// In contrast to the best-fit strategy, the worst-fit strategy searches for the largest 
/// available memory block. This can potentially lead to less fragmentation but may also 
/// result in less efficient memory usage.
/// 
/// # Next Fit:
/// 
/// Next fit is similar to first fit but remembers the last block searched in the linked 
/// list. It starts searching from the last block, which can help improve allocation locality.

use crate::single;
use super::SubAllocator;
use core::alloc::{Allocator, Layout, GlobalAlloc, AllocError};
use core::mem;
use core::ptr::{self, NonNull};
use core::sync::atomic::{AtomicUsize, Ordering, AtomicBool};

/// Start address of the memory heap. Use any address as long as it is not used.
pub const FREE_LIST_ALLOC_HEAP_START: usize = 0o_000_001_000_000_0000;
/// Maximal size of the whole arena. Adjust the size as needed.
pub const FREE_LIST_ALLOC_HEAP_ARENA: usize = 128 * 1024;
/// Constant size of the free-list node.
const NODE_HEADER_SIZE: usize = mem::size_of::<NodeHeader>();

/// Static default allocator instance
single! {
    pub mut FREE_LIST_ALLOC: FreeListAlloc = FreeListAlloc::new(
        FREE_LIST_ALLOC_HEAP_START,
        FREE_LIST_ALLOC_HEAP_START + FREE_LIST_ALLOC_HEAP_ARENA,
        SearchStrategy::FIRST_FIT,
    )
}

/// A structure that represents the node, within which the memory will be written.
#[derive(Debug)]
#[repr(C)]
struct NodeHeader {
    // The node's size must be known to fit the requested memory into the block.
    size: usize,
    // pointer to the next available list.
    next: AtomicUsize,
}

impl NodeHeader {
    #[inline(always)]
    fn new(self_ptr: usize, next_ptr: usize, size: usize) -> usize {
        let node = NodeHeader { size, next: AtomicUsize::new(next_ptr) };

        unsafe {
            ptr::write_unaligned(
                self_ptr as *mut NodeHeader, 
                node,
            )
        }
        
        self_ptr
    }

    #[inline(always)]
    fn ref_clone(&self) -> &mut Self {
        unsafe { (self as *const Self as *mut Self).as_mut().unwrap() }
    }
}

/// The structure of the free list allocator.
#[derive(Debug)]
pub struct FreeListAlloc {
    // A search strategy, currently used in the free list allocator.
    search_strategy: SearchStrategy,
    
    // Head pointer.
    head: AtomicUsize,

    heap_start: usize,
    heap_end: usize,

    // An extra pointer for next fit strategy.
    next_fit_ptr: AtomicUsize,
    // Will be false until the first allocation request arrives.
    initialized: AtomicBool,
}

impl FreeListAlloc {
    /// Creates a new instance of free list allocator.
    pub fn new(heap_start: usize, heap_end: usize, search_strategy: SearchStrategy) -> Self {
        Self {
            search_strategy: search_strategy,
            head: AtomicUsize::new(0),
            heap_start, heap_end,
            next_fit_ptr: AtomicUsize::new(0),
            initialized: AtomicBool::new(false),
        }
    }

    /// Changes the search strategy for the allocator.
    /// 
    /// This function must be used wisely to make allocations efficient.
    pub fn change_strategy(&mut self, ss: SearchStrategy) {
        self.search_strategy = ss
    }

    /// A debug function that prints out every single node that is currently in the list.
    pub fn info(&self) {
        let mut next_node = self.head.load(Ordering::Relaxed);
        let mut i = 1;

        while let Some(mut node) = unsafe { (next_node as *mut NodeHeader).as_mut() } {
            crate::print!("{}: ({:#x}), ", i, node as *const _ as usize);
            next_node = node.next.load(Ordering::Acquire);
            i += 1;
        }

        if !self.initialized.load(Ordering::Relaxed) {
            crate::println!(crate::Color::YELLOW; "The allocator is not initialized yet.");
        } else {
            crate::println!();
        }
    }
}

unsafe impl Allocator for FreeListAlloc {
    /// Allocates memory in a free node.
    /// 
    /// # Thread safety
    /// 
    /// ## This allocation algorithm is lock-free:
    /// 
    /// Every thread that wants to allocate memory, iterates via free memory list, gets the free node and
    /// splits it into two if the requested memory is smaller that obtained node and returns the splitted one
    /// that is capable to contain the requested layout. If the requested memory is the same as the node can
    /// contain, it just returns the node and marks it as used.
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        // Calculate a mask to enforce the required alignment.
        let align_mask = !(layout.align() - 1);

        // The main loop for catching the current head.
        'main: loop {
            let current_head = if self.search_strategy == SearchStrategy::NEXT_FIT {
                self.next_fit_ptr.load(Ordering::Relaxed)
            } else {
                self.head.load(Ordering::Relaxed)
            };

            // Trying to fetch a head node. It can only fail if the head was changed in between
            // those two operations or the first node is not initialized yet.
            if let Some(mut node) = unsafe { (current_head as *mut NodeHeader).as_mut() } {
                /// We need the previous node to compare and pointer swapping.
                let mut prev_node = node.ref_clone();
                /// This variable will only be used when best fit or worst fit mode is enabled.
                let mut fit_node = node.ref_clone();
                
                // Inner loop for searching within the nodes.
                'inner: loop {
                    use SearchStrategy::*;
                    // Based on the selected strategy, the inner code will vary.
                    match self.search_strategy {
                        FIRST_FIT | NEXT_FIT => {
                            if node.size > layout.size() {
                                let new_node = NodeHeader::new(
                                    ((node as *const NodeHeader as usize + 1) + NODE_HEADER_SIZE + layout.size()) & align_mask,
                                    node.next.load(Ordering::Relaxed),
                                    node.size - layout.size(),
                                );

                                if let Err(_) = prev_node.next.compare_exchange(
                                    node as *const _ as usize,
                                    new_node,
                                    Ordering::SeqCst,
                                    Ordering::Relaxed,
                                ) {
                                    if let Err(_) = prev_node.next.compare_exchange(
                                        0,
                                        new_node,
                                        Ordering::SeqCst,
                                        Ordering::Relaxed,
                                    ) {
                                        unsafe { ptr::drop_in_place(new_node as *mut NodeHeader); }
                                        continue 'main
                                    }
                                }

                                break 'inner
                            } else if node.size == layout.size() {
                                let next_node = node.next.load(Ordering::Relaxed);
                                if let Err(_) = prev_node.next.compare_exchange(
                                    node as *const _ as usize,
                                    next_node,
                                    Ordering::SeqCst,
                                    Ordering::Relaxed,
                                ) {
                                    if let Err(_) = prev_node.next.compare_exchange(
                                        0,
                                        next_node,
                                        Ordering::SeqCst,
                                        Ordering::Relaxed,
                                    ) {
                                        unsafe { ptr::drop_in_place(next_node as *mut NodeHeader); }
                                        continue 'main
                                    }
                                }

                                break 'inner
                            }
                        },
                        BEST_FIT => {
                            if node.size == layout.size() {
                                let next_node = node.next.load(Ordering::Relaxed);
                                if let Err(_) = prev_node.next.compare_exchange(
                                    node as *const _ as usize,
                                    next_node,
                                    Ordering::SeqCst,
                                    Ordering::Relaxed,
                                ) {
                                    if let Err(_) = prev_node.next.compare_exchange(
                                        0,
                                        next_node,
                                        Ordering::SeqCst,
                                        Ordering::Relaxed,
                                    ) {
                                        unsafe { ptr::drop_in_place(next_node as *mut NodeHeader); }
                                        continue 'main
                                    }
                                }

                                break 'inner
                            } else if node.size > layout.size() {
                                let next_node = node.next.load(Ordering::Relaxed);
                                
                                if node.size < prev_node.size {
                                    fit_node = node.ref_clone();
                                }

                                if next_node == 0 {
                                    let new_node = NodeHeader::new(
                                        ((fit_node as *const NodeHeader as usize + 1) + NODE_HEADER_SIZE + layout.size()) & align_mask,
                                        fit_node.next.load(Ordering::Relaxed),
                                        fit_node.size - layout.size(),
                                    );
    
                                    if let Err(_) = prev_node.next.compare_exchange(
                                        fit_node as *const _ as usize,
                                        new_node,
                                        Ordering::SeqCst,
                                        Ordering::Relaxed,
                                    ) {
                                        if let Err(_) = prev_node.next.compare_exchange(
                                            0,
                                            new_node,
                                            Ordering::SeqCst,
                                            Ordering::Relaxed,
                                        ) {
                                            unsafe { ptr::drop_in_place(new_node as *mut NodeHeader); }
                                            continue 'main
                                        }
                                    }

                                    node = fit_node;
                                    break 'inner
                                }
                            }
                        }
                        _ => unimplemented!(),
                    }

                    prev_node = node.ref_clone();
                    node = unsafe {
                        if let Some(next_node) = (node.next.load(Ordering::SeqCst) as *mut NodeHeader).as_mut() {
                            next_node
                        } else {
                            if self.search_strategy == NEXT_FIT {
                                if let Ok(_) = self.next_fit_ptr.compare_exchange(
                                    self.next_fit_ptr.load(Ordering::Acquire),
                                    self.head.load(Ordering::Acquire),
                                    Ordering::SeqCst,
                                    Ordering::Relaxed,
                                ) {
                                    continue 'main
                                }
                            }

                            return Err(AllocError)
                        }
                    }
                }

                // If this cas operation will fail, it will only mean that some other thread
                // did it first or the current node is not a head, therefore it must be failed.
                self.head.compare_exchange(
                    node as *const _ as usize,
                    node.next.load(Ordering::Relaxed),
                    Ordering::SeqCst,
                    Ordering::Relaxed,
                );

                if self.search_strategy == SearchStrategy::NEXT_FIT {
                    self.next_fit_ptr.compare_exchange(
                        self.next_fit_ptr.load(Ordering::Acquire),
                        node.next.load(Ordering::Relaxed),
                        Ordering::SeqCst,
                        Ordering::Relaxed,
                    );
                }
                
                let return_ptr = (node as *const _ as usize + NODE_HEADER_SIZE) & align_mask;
                
                #[cfg(debug_assertions)] {
                    crate::println!("Allocating {} bytes at {:#x}", layout.size(), return_ptr);
                }
                
                return Ok(NonNull::slice_from_raw_parts(
                    NonNull::new(return_ptr as *mut u8).unwrap(),
                    layout.size(),
                ));
            } else {
                // This condition will be only called once at the first allocation. This ensures that
                // next allocations will be faster and will not require to check this every time.
                if let Ok(_) = self.initialized.compare_exchange(
                    false,
                    true,
                    Ordering::SeqCst,
                    Ordering::Relaxed,
                ) {
                    let _ = NodeHeader::new(
                        self.heap_start & align_mask,
                        0,
                        self.arena_size(),
                    );

                    self.head.compare_exchange(
                        0,
                        self.heap_start,
                        Ordering::SeqCst,
                        Ordering::Relaxed,
                    );

                    self.next_fit_ptr.compare_exchange(
                        0,
                        self.heap_start,
                        Ordering::SeqCst,
                        Ordering::Relaxed,
                    );
                }
            }
        }
    }

    /// Deallocates the memory by setting the given node as unused.
    /// 
    /// It also plays the role as a 'merger' function. Since it must search through the list until it 
    /// finds the needed node, it will also merge the unused nodes into one big one for further use.
    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        let free_node = {
            ((ptr.as_ptr() as usize).saturating_sub(NODE_HEADER_SIZE) as *mut NodeHeader)
        };
        let next_node_ptr = free_node.as_mut().unwrap().next.load(Ordering::Relaxed);

        loop {
            let mut next_node = self.head.load(Ordering::Relaxed);
            // crate::println!("FREE NODE: {:#x}, WE ARE DEALLOCATING: {:#x}, AND THE NEXT ONE IS {:#x}", free_node as usize, next_node_ptr, next_node);

            if next_node == next_node_ptr {
                self.head.compare_exchange(
                    next_node_ptr,
                    free_node as usize,
                    Ordering::SeqCst,
                    Ordering::Relaxed,
                );
            }

            let mut prev_node = free_node.as_mut().unwrap();
            while let Some(mut node) = unsafe { (next_node as *mut NodeHeader).as_mut() } {
                let node_addr = node as *const _ as usize;

                if let Ok(_) = node.next.compare_exchange(
                    next_node_ptr,
                    free_node as usize,
                    Ordering::SeqCst,
                    Ordering::Relaxed,
                ) {
                    break
                }

                if let Ok(_) = prev_node.next.compare_exchange(
                    node_addr,
                    node.next.load(Ordering::Acquire),
                    Ordering::SeqCst,
                    Ordering::Relaxed,
                ) {
                    prev_node.size += node.size;
                    next_node = node.next.load(Ordering::Acquire);
                    ptr::drop_in_place(node_addr as *mut NodeHeader);
                    
                    continue
                }

                prev_node = node.ref_clone();
                next_node = node.next.load(Ordering::Acquire);
            }
    
            #[cfg(debug_assertions)] {
                crate::println!("Deallocating {} bytes from {:#x}", layout.size(), ptr.as_ptr() as usize);
            }
    
            break
        }
    }
}

impl SubAllocator for FreeListAlloc {
    fn arena_size(&self) -> usize {
        self.heap_end - self.heap_start
    }

    fn heap_addr(&self) -> usize {
        self.heap_start
    }
}

/// Search strategies that allocator can use.
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchStrategy {
    /// This search strategy looks for the first available memory block in the linked list that 
    /// is large enough to satisfy the allocation request. It stops searching once it finds a 
    /// suitable block. This strategy is simple but may lead to memory fragmentation.
    FIRST_FIT,
    /// The best-fit strategy searches for the smallest available memory block that can accommodate
    /// the allocation request. It aims to minimize fragmentation but may require more time to 
    /// search for the best-fitting block.
    BEST_FIT,
    /// In contrast to the best-fit strategy, the worst-fit strategy searches for the largest 
    /// available memory block. This can potentially lead to less fragmentation but may also 
    /// result in less efficient memory usage.
    WORST_FIT,
    /// Next fit is similar to first fit but remembers the last block searched in the linked 
    /// list. It starts searching from the last block, which can help improve allocation locality.
    NEXT_FIT,
}