/// Buddy allocator implementation.
///
/// This allocator is used to prevent memory segmentation by diving each
/// memory segment by two, until the requested data size is achieved or
/// until the next division will not fit the requested data.
///
/// Since most datatypes are some multiples by two, this allows the allocator
/// to satisfy deallocations that happen rapidly as well as fast allocations.
///
/// This particular implementation is a binary allocator system. Which means
/// that each block will be divided by two same halves. Each 'Buddy' in such
/// system will give info about it's children, if they are blocked or not.
///
/// # Fragmentation
///
/// This allocator allows for the least amount of fragmantation, because it is
/// holding all it's data within a fixed sized chunks, which are a powers of two,
/// if the header is counted.
///
/// # Warning
///
/// This implementation makes the smallest allocation possible of 64 bytes. Because
/// the 'BuddyHeader' structure itself has a size of 24 bytes, all data which are smaller
/// or equal to 32 bytes must be written within a 64 byte node. Larger data will be written
/// on bigger nodes.

use crate::single;
use super::SubAllocator;
use core::alloc::{Allocator, Layout, GlobalAlloc, AllocError};
use core::mem;
use core::ptr::{self, NonNull};
use core::sync::atomic::{AtomicUsize, Ordering, AtomicBool};

/// Start address of the memory heap. Use any address as long as it is not used.
pub const BUDDY_ALLOC_HEAP_START: usize = 0o_000_001_000_000_0000;
/// Maximal size of the whole arena. Adjust the size as needed.
pub const BUDDY_ALLOC_HEAP_ARENA: usize = 128 * 1024;
/// Constant size of the free-list node.
const BUDDY_HEADER_SIZE: usize = mem::size_of::<BuddyHeader>();

single! {
    pub mut BUDDY_ALLOC: BuddyAlloc = BuddyAlloc::new(
        BUDDY_ALLOC_HEAP_START,
        BUDDY_ALLOC_HEAP_START + BUDDY_ALLOC_HEAP_ARENA,
    );
}

/// A single node of the buddy allocator's binary tree
///
/// Like in regular binary tries, each node points to their
/// two neighbors. If pointers are null pointers, that means
/// this node is a leaf. The struct itself is pretty bulky, but
/// it is a must, otherwise, the main algorithm will be changed.
#[derive(Debug)]
#[repr(C)]
struct BuddyHeader {
    // pointer to the right block, if exist.
    right: AtomicUsize,
    // pointer to the left block, if exist.
    left: AtomicUsize,
    // A current status of the buddy.
    status: BuddyStatus,
}

impl BuddyHeader {
    #[inline(always)]
    fn new(self_ptr: usize, left: usize, right: usize) -> usize {
        let node = BuddyHeader {
            left: AtomicUsize::new(left),
            right: AtomicUsize::new(right),
            status: BuddyStatus::FREE,
        };

        unsafe {
            ptr::write_unaligned(
                self_ptr as *mut BuddyHeader, 
                node,
            )
        }
        
        self_ptr
    }

    #[inline(always)]
    fn split(&mut self, arena_size: usize, align_mask: usize, size: usize) {
        self.status = BuddyStatus::LEFT; // Starting with left first.

        // Allocating the left buddy.
        let lptr = (self as *const BuddyHeader as usize) + BUDDY_HEADER_SIZE; 
        // Allocating the right buddy.
        let rptr = lptr + (size / 2); 

        let left_buddy = BuddyHeader::new(
            lptr & align_mask,
            0, 0,
        );

        let right_buddy = BuddyHeader::new(
            rptr & align_mask,
            0, 0,
        );

        // Now we can change the pointers.
        self.left.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |_| Some(left_buddy));
        self.right.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |_| Some(right_buddy));
    }

    #[inline(never)]
    unsafe fn merge(&mut self, status: BuddyStatus, merge_ptr: usize, size: usize) -> Result<(&mut BuddyHeader, &mut BuddyHeader), ()> {
        'main: loop {
            let side = match status {
                BuddyStatus::RIGHT => self.right.load(Ordering::Acquire),   // Going right.
                BuddyStatus::LEFT => self.left.load(Ordering::Acquire),     // Going left.
                BuddyStatus::BLOCKED => self as *const _ as usize,          // Going for the head.
                _ => unreachable!(),
            };

            if let Some(next_buddy) = unsafe { (side as *mut BuddyHeader).as_mut() } {
                // Checking for the requested buddy.
                if side == merge_ptr {
                    return Ok((self, next_buddy))
                }

                // Searching the required node.
                if merge_ptr < (side + BUDDY_HEADER_SIZE + size / 2 + 10) {
                    if let Ok((parent_node, left)) = next_buddy.merge(BuddyStatus::LEFT, merge_ptr, size / 2) {
                        // Checking the right node.
                        if let Some(right) = unsafe { (parent_node.right.load(Ordering::Acquire) as *mut BuddyHeader).as_mut() } {
                            if right.status == BuddyStatus::FREE {
                                // It is ok to make this one free at that point.
                                parent_node.status = BuddyStatus::FREE;
                                // We own them right now, so dropping them is ok
                                ptr::drop_in_place(left as *mut BuddyHeader);
                                ptr::drop_in_place(right as *mut BuddyHeader);

                                // Getting higher in a hierarchy.
                                return Ok((self, parent_node))
                            }
                        }
                    }
                    return Err(())
                } else {
                    if let Ok((parent_node, right)) = next_buddy.merge(BuddyStatus::RIGHT, merge_ptr, size / 2) {
                        // Checking the right node.
                        if let Some(left) = unsafe { (parent_node.left.load(Ordering::Acquire) as *mut BuddyHeader).as_mut() } {
                            if left.status == BuddyStatus::FREE {
                                // It is ok to make this one free at that point.
                                parent_node.status = BuddyStatus::FREE;
                                // We own them right now, so dropping them is ok
                                ptr::drop_in_place(left as *mut BuddyHeader);
                                ptr::drop_in_place(right as *mut BuddyHeader);

                                // Getting higher in a hierarchy.
                                return Ok((self, parent_node))
                            }
                        }
                    }
                    return Err(())
                }
            } else {
                return Err(())
            }

        }
    }

    #[inline(never)]
    fn search(&mut self, mut status: BuddyStatus, arena_size: usize, alloc_size: usize, size: usize) -> Result<&mut BuddyHeader, ()> {
        // If the node is divided, checking the left side first and then the right
        // one. Other splitted nodes are in high priority, as they do not require
        // further splitting for buddies.

        let mut tries = 0; 
        'main: loop {
            let mut side = match status {
                BuddyStatus::RIGHT => self.right.load(Ordering::Acquire),
                BuddyStatus::LEFT => self.left.load(Ordering::Acquire),
                _ => unreachable!(),
            };

            // Amount of tries before exiting this block.
            if tries > 2 {
                break 'main
            }

            if let Some(next_buddy) = unsafe { (side as *mut BuddyHeader).as_mut() } {
                match next_buddy.status {
                    l @ BuddyStatus::LEFT => {
                        if (size / 2).saturating_sub(BUDDY_HEADER_SIZE) >= alloc_size { 
                            // Going to all different leaves until finding siutable place.
                            if let Ok(n) = next_buddy.search(l, arena_size, alloc_size, size/2) {
                                self.status = BuddyStatus::RIGHT;
                                return Ok(n)
                            }
                        }
                        status = BuddyStatus::RIGHT; // Trying to change side.
                        tries += 1;
                    },
                    r @ BuddyStatus::RIGHT => {
                        if (size / 2).saturating_sub(BUDDY_HEADER_SIZE) >= alloc_size { 
                            // Going to all different leaves until finding siutable place.
                            if let Ok(n) = next_buddy.search(r, arena_size, alloc_size, size/2) {
                                self.status = BuddyStatus::LEFT; 
                                return Ok(n)
                            }
                        }
                        status = BuddyStatus::LEFT; // Trying to change side.
                        tries += 1;
                    }, 
                    BuddyStatus::FREE => {
                        return Ok(next_buddy);
                    }, // We cannot go wrong with a free one.
                    BuddyStatus::BLOCKED => {
                        if status != BuddyStatus::RIGHT {
                            return self.search(BuddyStatus::RIGHT, arena_size, alloc_size, size/2)
                        } else {
                            break 'main
                        }
                    },
                }
            } else {
                // This can only happen in a multi-threaded environment. Repeating.
                continue 'main
            }
        }

        return Err(())
    }
}

/// Three states in which a buddy can be.
///
/// Each status gives a proper info to the allocator,
/// so new allocations will appear in right places.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BuddyStatus {
    /// Basically mean a whole free clean block.
    FREE,
    /// Tells the allocator to search on the left.
    LEFT,
    /// Tells the allocator to search on the right.
    RIGHT,
    /// Tells the allocator that both buddies are allocated.
    BLOCKED,
}

/// Buddy allocator structure.
///
/// This structure is an interface for special concurrent
/// binary tree, which is used to define memory regions.
///
/// The allocator is using a buddy allocator's algorithm,
/// which means it will divide it's memory regions by two,
/// until the best possible memory region is found for the
/// requested data size. The binary tree provides a fast way
/// to allocate the data, as well as providing a waterfall-like
/// method to deallocate unused regions.
#[derive(Debug)]
pub struct BuddyAlloc {
    heap_start: usize,
    heap_end: usize,
    // Pointer to the biggest node, which is closer to the root. 
    head: AtomicUsize,
    // Will be false until the first allocation request arrives.
    initialized: AtomicBool,
}

impl BuddyAlloc {
    /// Creates a new instance of free list allocator.
    pub fn new(heap_start: usize, heap_end: usize) -> Self {
        Self {
            head: AtomicUsize::new(0),
            heap_start, heap_end,
            initialized: AtomicBool::new(false),
        }
    }
}

unsafe impl Allocator for BuddyAlloc {
    /// Allocates memory in a free node.
    /// 
    /// # Thread safety
    /// 
    /// ## This allocation algorithm is lock-free:
    ///
    /// Only one writer can split the FREE node into two parts. No readers would be able to
    /// move on until the node is splitted apart by the writer.
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        // Calculate a mask to enforce the required alignment.
        let align_mask = !(layout.align() - 1);

        let mut size = self.arena_size();
        // Trying to obtain the head.
        //
        // This can only fail at the very first allocation, where there is still no
        // main header. This will also fail, if there is no more memory to allocate.
        'main: loop {
            if let Some(mut node) = unsafe { (self.head.load(Ordering::Acquire) as *mut BuddyHeader).as_mut() } {
                'inner: loop {
                    size /= 2;
                    match node.status {
                        BuddyStatus::FREE => {
                            // If no further division is possible, allocating the buddy.
                            if (size/2).saturating_sub(BUDDY_HEADER_SIZE) < layout.size() {
                                // Marking as used.
                                node.status = BuddyStatus::BLOCKED;

                                let return_ptr = (node as *const _ as usize + BUDDY_HEADER_SIZE) & align_mask;

                                #[cfg(debug_assertions)] {
                                    crate::println!("Allocating {} bytes at {:#x}, with node size of: {} bytes.", layout.size(), return_ptr, size);
                                }

                                return Ok(NonNull::slice_from_raw_parts(
                                    NonNull::new(return_ptr as *mut u8).unwrap(),
                                    layout.size(),
                                ));
                            } else {
                                // If possible, dividing in half, and repeating.
                                node.split(self.arena_size(), align_mask, size);
                                continue 'inner
                            }
                        },
                        s @ (BuddyStatus::LEFT | BuddyStatus::RIGHT) => {
                            // Calling the inner function of the node, which will be recursive.
                            if let Ok(n) = node.search(s, self.arena_size(), layout.size(), size) {
                                node = n;
                                continue 'inner
                            }

                            // The node is splitted, but the allocation is too big for both sides.
                            return Err(AllocError) 
                        },
                        BuddyStatus::BLOCKED => {
                            // At this point the whole heap is full or fragmented.
                            return Err(AllocError)
                        }     
                    }
                }
            } else {
                // This condition will be only called once at the first allocation. This ensures that
                // next allocations will be faster and will not require to check this every time.
                if let Ok(_) = self.initialized.compare_exchange(
                    false,
                    true,
                    Ordering::SeqCst,
                    Ordering::Relaxed,
                ) {
                    let _ = BuddyHeader::new(
                        self.heap_start & align_mask,
                        0, 0,
                    );

                    self.head.compare_exchange(
                        0,
                        self.heap_start,
                        Ordering::SeqCst,
                        Ordering::Relaxed,
                    );
                }
            }
        }

        // This part must be unreachable.
        unreachable!("Breaked out of the main loop.");
    }

    /// Deallocates the memory by setting the given node as unused.
    ///
    /// While searches for the requested memory from the top, merges all buddies, which
    /// are freed, including the one which is about to be freed by this function call.
    ///
    /// TODO!
    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        #[cfg(debug_assertions)] {
            crate::println!("Deallocating {} bytes from {:#x}", layout.size(), ptr.as_ptr() as usize);
        }

        let node_ptr = (ptr.as_ptr() as usize).saturating_sub(BUDDY_HEADER_SIZE);

        // Marking the node as freed.
        if let Some(n) = (node_ptr as *mut BuddyHeader).as_mut() {
            n.status = BuddyStatus::FREE;
        }

        if let Some(mut node) = unsafe { (self.head.load(Ordering::Acquire) as *mut BuddyHeader).as_mut() } {
            // Merging the node.
            node.merge(BuddyStatus::BLOCKED, node_ptr as usize, self.arena_size());
        }
    }
}

impl SubAllocator for BuddyAlloc {
    fn arena_size(&self) -> usize {
        self.heap_end - self.heap_start
    }

    fn heap_addr(&self) -> usize {
        self.heap_start
    }
}
