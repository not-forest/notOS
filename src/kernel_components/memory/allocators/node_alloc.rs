/// Node allocator implementation
/// 
/// This is a fast allocator that can deallocate memory for further
/// reuse, while not overheading the system with hard algorithms, but
/// sacrificing the stack memory to control every single node of memory. 
/// This allocator holds every memory node inside the stack and marks them 
/// as used or not. It has to know exactly the size of each node, that can 
/// be chosen only at compile time.
/// 
/// Both allocation and deallocation is supported and can be used in systems
/// that work rapidly with DST's, in the cost of stack memory.
/// 
/// # Important
/// 
/// The amount of nodes must be decided at compile time.
/// 
/// # Scale
/// 
/// This allocator must have an incredible small heap, or incredibly big node size.
/// 
/// For better understanding examine this allocator config:
/// const NODE_AMOUNT: usize = 128; 
/// const NODE_SIZE: usize = 4; // 4 bytes for one node.
/// 
/// From this constants we get that:
/// pub const NODE_ALLOC_HEAP_ARENA: usize = NODE_AMOUNT * NODE_SIZE; // = 512 bytes of heap memory.
/// 
/// However the stack memory for this allocator will be:
/// size_of::<MemoryNode> = 16;
/// STACK_SIZE = 16 * 128 = 2048.
/// 
/// To make heap memory actually bigger than used stack memory, the node size must be bigger than 16 bytes.
/// However this would lead to big amount of external fragmentation, when allocating a lot of small objects,
/// so it is better to use this allocator when you need a fast, but small heap.

use crate::single;
use super::SubAllocator;
use core::alloc::{Allocator, Layout, AllocError};
use core::mem::{self, MaybeUninit};
use core::ptr::{self, NonNull};
use core::sync::atomic::{AtomicUsize, AtomicBool, Ordering};
use core::cell::UnsafeCell;
use core::marker::PhantomData;

/// Start address of the memory heap. Use any address as long as it is not used.
pub const NODE_ALLOC_HEAP_START: usize = 0o_000_001_000_000_0000;
/// Maximal size of the whole arena.
pub const NODE_ALLOC_HEAP_ARENA: usize = NODE_AMOUNT * NODE_SIZE;
/// Overall amount of nodes in the allocator. The arena size of allocator will be decided as NODE_AMOUNT * NODE_SIZE.
const NODE_AMOUNT: usize = 128;
/// The size of a single node. When smaller number is used, the external fragmentation is getting less frequent, but the
/// overall performance, will decrease.
const NODE_SIZE: usize = 4;

/// Static default allocator instance
single! {
    pub NODE_ALLOC: NodeAlloc = NodeAlloc::new(
        NODE_ALLOC_HEAP_START,
        NODE_SIZE,
    );
}

/// Implementation of node allocator.
/// 
/// A simple memory allocation algorithm that deallocates memory fast with the use of
/// stack memory to contain every single memory node within.
/// 
/// # Initialization
/// 
/// Initializes a pointer to the first node. Every single node is initialized at compile time.
/// 
/// # Allocation
/// 
/// When a memory allocation request comes in returns the pointer to the next node and marks it as
/// used. If several nodes needed to contain one object, all of them will be marked as used.
/// 
/// # Deallocation
/// 
/// Deallocates memory by marking the nodes as unused. The next allocation will be within those nodes,
/// if they do fit inside.
/// 
/// # Fragmentation
/// 
/// This allocator has several problems with external and internal fragmentation. If the object is
/// smaller than the node, entire node will still be marked as used. This can lead to internal fragmentation
/// within the node, because the area needed can be bigger than requested. If many small objects will take nodes
/// in non linear order, it can lead to external fragmentation for bigger objects, which cannot fit between gaps.
/// When using this allocator, the node size must be chosen wisely, as the most expected memory chunk size, used
/// is the system. (If the OS will not use big memory chunks, the allocator will overflow fast, with some
/// small sized data, like integers. When deciding which size to use, it should be close to the memory size
/// of the most used data type in your system.). Making the node size too small, can lead to storing too much
/// data on the stack and worse performance.
/// 
/// # Thread safety
/// 
/// Both allocation and deallocation are lock-free and thread safe. More info on allocate and deallocate.
#[derive(Debug)]
pub struct NodeAlloc {
    // The array located on stack, that holds information about every single memory node.
    node_array: [MemoryNode; NODE_AMOUNT],
    // The size of one node.
    node_size: usize,
}

impl NodeAlloc {
    /// Creates a new node allocator instance.
    pub fn new(heap_start: usize, node_size: usize) -> Self {
        let array = {
            let mut array: [MaybeUninit<MemoryNode>; NODE_AMOUNT] = unsafe { MaybeUninit::uninit().assume_init() };
            
            for (i, node) in array.iter_mut().enumerate() {
                node.write(MemoryNode::new(heap_start + node_size * i));
            }

            unsafe { mem::transmute::<_, [MemoryNode; NODE_AMOUNT]>(array) }
        };

        Self {
            node_array: array,
            node_size,
        }
    }
}

unsafe impl Allocator for NodeAlloc {
    /// Allocates memory for DST by searching for free nodes in the node array.
    /// 
    /// # Thread safety
    /// 
    /// ## This allocation algorithm is lock-free:
    /// 
    /// Every thread would first find the area that is suitable for the requested memory
    /// size, then try to mark all the nodes, in which the object will be placed, as used.
    /// 
    /// If the thread manages to do it before the other threads, it will obtain the pointer,
    /// to allocated memory nodes. If not, it will clean all the wrong marked allocations,
    /// and retry the whole process again.
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        // Calculate a mask to enforce the required alignment.
        let align_mask = !(layout.align() - 1);

        // The main loop that tries to obtain memory before other threads do so first.
        'main: loop {
            // Size left to allocate.
            let mut leftover_size = layout.size();
            let mut id = self.node_array.len();

            for node in self.node_array.iter().rev() {

                if id + layout.size() - 1 / self.node_size >= self.node_array.len() {
                    id -= 1;
                    continue
                }

                // Check if the node is already in use.
                if !node.used.load(Ordering::Acquire) {
                    if leftover_size <= self.node_size {
                        let mut cas_counter = 1;

                        // Checking array bounds.
                        let right_id = if layout.size() > self.node_size {
                                                layout.size() / self.node_size
                                            } else { 1 };

                        // Trying to change every single node flag that we want to allocate.
                        for current_node in self.node_array[id - 1 .. id + right_id - 1].iter() {
                            // If we encounter an error while doing cas operations, it would only mean that the other thread
                            // is allocated this memory node faster that us. Therefore, we must clean up those fake allocations,
                            // that we did, and try again from the very start.
                            if let Err(_) = current_node.used.compare_exchange(
                                false,
                                true,
                                Ordering::SeqCst,
                                Ordering::Relaxed,
                            ) {
                                // Since we already made those fake allocations ourselves, we own them, therefore
                                // we can easily clean them, without the need of cas operations.
                                for broken_node in self.node_array[id - 1 .. id + cas_counter - 1].iter() {
                                    broken_node.used.store(false, Ordering::Release)
                                }

                                continue 'main
                            }
                            cas_counter += 1;
                        }

                        #[cfg(debug_assertions)] {
                            crate::println!("Allocating {} bytes at {:#x}", layout.size(), node.addr);
                        }

                        // After all nodes are noted as used, return the pointer to the first node.
                        return Ok(NonNull::slice_from_raw_parts(
                            NonNull::new((node.addr & align_mask) as *mut u8).unwrap(),
                            layout.size(),
                        ));
                    }

                    leftover_size -= self.node_size
                }

                id -= 1;
            }

            return Err(AllocError)
        }
    }

    /// Allocates memory for DST by marking the node, in which the object was placed before, as unused.
    /// 
    /// # Thread safety
    /// 
    /// ## This allocation algorithm is lock-free:
    /// 
    /// The thread will try to mark every single used node, within the allocated region as unused,
    /// with the use of cas operations. If it does fail, then some other thread did it first, and we
    /// must just do nothing.
    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        let start_id = (ptr.as_ptr() as usize - NODE_ALLOC_HEAP_START) / self.node_size; 
        let end_id = start_id + layout.size() / self.node_size;
        
        #[cfg(debug_assertions)] {
            crate::println!("Deallocating {} bytes from {:#x}", layout.size(), ptr.as_ptr() as usize);
        }

        // Since we own the allocated memory region, we can simply deallocate the memory
        // nodes with cas operations, and it would fail only when other thread did so first.
        for node in self.node_array[start_id..end_id + 1].iter() {
            if let Err(_) = node.used.compare_exchange(
                true,
                false,
                Ordering::SeqCst,
                Ordering::Relaxed
            ) {
                break
            }
        }
    }
}

impl SubAllocator for NodeAlloc {
    fn arena_size(&self) -> usize {
        NODE_ALLOC_HEAP_ARENA
    }

    fn heap_addr(&self) -> usize {
        NODE_ALLOC_HEAP_START
    }
}

/// A single node that says info about the current state of memory.
#[derive(Debug)]
struct MemoryNode {
    // Virtual memory address of the current node in the heap.
    addr: usize,
    // This flag will be set to true, when the allocator, is ready to give the memory
    // pointer.
    used: AtomicBool,
}

impl MemoryNode {
    pub fn new(addr: usize) -> Self {
        Self { addr, used: AtomicBool::new(false) }
    }
}
