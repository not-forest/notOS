/// A concurrent queue implementation module.

use crate::kernel_components::memory::allocators::GAllocator;
use core::sync::atomic::{AtomicUsize, Ordering, AtomicBool};
use core::alloc::{GlobalAlloc, Allocator, Layout};
use core::mem::{self, MaybeUninit};
use core::marker::PhantomData;
use core::ptr::{self, NonNull};

/// A concurrent queue which uses Michael & Scott lock-free algorithm.
#[derive(Debug)]
pub struct ConcurrentQueue<T, A = GAllocator> where A: Allocator + 'static {
    head: AtomicUsize,
    tail: AtomicUsize,
    alloc: &'static mut A,
    _marker: PhantomData<T>,
}

impl<T, A: Allocator> ConcurrentQueue<T, A> {
    /// Creates a new instance of 'ConcurrentQueue'.
    /// 
    /// # Dummy
    /// 
    /// This instance will automatically insert a dummy node inside, to make lock-free
    /// algorithm possible.
    pub fn new(alloc: &'static mut A) -> Self {
        let dummy  = ConcurrentQueueNode::<T>::dummy();
        let ptr = dummy.node_alloc(alloc);

        Self {
            head: AtomicUsize::new(ptr as usize),
            tail: AtomicUsize::new(ptr as usize),
            alloc: alloc,
            _marker: PhantomData,
        }
    }

    /// Works like new and creates a new instance of 'ConcurrentQueue', but with no dummy node.
    /// 
    /// The content must be provided, as the first element of the queue, because it has to be
    /// used instead of the dummy.
    pub fn new_non_dummy(content: T, alloc: &'static mut A) -> Self {
        let node = ConcurrentQueueNode { 
            data: content,
            next: AtomicUsize::new(0),
        };
        
        let ptr = node.node_alloc(alloc);

        Self {
            head: AtomicUsize::new(ptr as usize),
            tail: AtomicUsize::new(ptr as usize),
            alloc: alloc,
            _marker: PhantomData,
        }
    }

    /// Enqueues the new content to the end of the list concurrently.
    /// 
    /// The algorithm is lock free and works on several individual CASes.
    pub fn enqueue(&mut self, content: T) {
        /// Allocating the node straight on, because we must enqueue the node no matter what.
        let node = ConcurrentQueueNode { 
            data: content,
            next: AtomicUsize::new(0),
        };

        node.node_alloc(self.alloc);

        loop {
            if let Some(tail) = unsafe { 
                (self.tail.as_ptr() as *mut ConcurrentQueueNode<T>).as_mut() 
            } {
                let next = tail.next.load(Ordering::Relaxed);
                if next == 0 {
                    // We must link the tail node with our new one. Repeating the process until it's done.
                    if let Ok(_) = tail.next.compare_exchange(
                        tail.next.load(Ordering::Acquire),
                        next,
                        Ordering::SeqCst,
                        Ordering::Relaxed,
                    ) {
                        // Trying to change the tail to the current real tail. If this fails, 
                        // it means some new item was enqueued faster.    
                        self.tail.compare_exchange(
                            self.tail.load(Ordering::Acquire),
                            tail as *const _ as usize,
                            Ordering::SeqCst,
                            Ordering::Relaxed,
                        );

                        break
                    }
                } else {
                    // Trying to swing the tail to the next node.
                    self.tail.compare_exchange(
                        self.tail.load(Ordering::Acquire),
                        tail as *const _ as usize,
                        Ordering::SeqCst,
                        Ordering::Relaxed,
                    );
                }
            }
        }
    }

    /// Dequeues the content from the queue.
    /// 
    /// This function returns the content from the queue if it not empty. Returns None otherwise.
    /// This algorithm is lock-free and it is based on CAS operations.
    pub fn dequeue(&mut self) -> Option<T> {
        loop {
            if let Some(head) = unsafe { 
                (self.head.as_ptr() as *mut ConcurrentQueueNode<T>).as_mut() 
            } {
                let tail = self.tail.load(Ordering::Relaxed);
                let next = head.next.load(Ordering::Relaxed);
                // Check №1: The head is equal to tail. This can mean that the tail is outdated
                // or that the queue is empty.
                if head as *const _ as usize == tail {
                    // Check №2: The head points to nowhere. Returning None
                    if next == 0 {
                        return None
                    }
                    // Trying to swing the tail to the next node, because it is outdated.
                    self.tail.compare_exchange(
                        self.tail.load(Ordering::Acquire),
                        tail,
                        Ordering::SeqCst,
                        Ordering::Relaxed,
                    );
                } else {
                    if let Ok(_) = self.head.compare_exchange(
                        self.head.load(Ordering::Acquire),
                        next,
                        Ordering::SeqCst,
                        Ordering::Relaxed,
                    ) {
                        // Deallocating the previous head and returning data.
                        let data = head.obtain();
                        head.node_dealloc(self.alloc);
                        return Some(data);
                    }
                }
            }
        }
    }
}

/// A single node for concurrent queue that is being allocated on the heap.
/// 
/// This struct does not use the exists bool flag, like the concurrent list does, because
/// it will never be read backwards.
#[derive(Debug)]
pub struct ConcurrentQueueNode<T> {
    /// Data that lies within the node. 
    data: T,
    /// Pointer value to the next node.
    next: AtomicUsize,
}

impl<T> ConcurrentQueueNode<T> {
    /// Returns the data.
    #[inline]
    fn obtain(&mut self) -> T {
        self.data
    }

    /// Just allocates the node to some random place on the heap.
    /// 
    /// Returns a pointer to the location.
    #[inline]
    fn node_alloc<A>(&self, alloc: &A) -> *mut Self where A: Allocator {
        let content_size = mem::size_of::<ConcurrentQueueNode<T>>();
        let content_align = mem::align_of::<ConcurrentQueueNode<T>>();
        let layout = Layout::from_size_align(content_size, content_align).unwrap();
        let ptr = unsafe { alloc.allocate(layout) }.unwrap().as_mut_ptr() as *mut ConcurrentQueueNode<T>;

        unsafe {
            ptr.write(*self);
        };

        ptr
    }

    // Deallocates the node at given ptr
    #[inline]
    fn node_dealloc<A>(&mut self, alloc: &A) where A: Allocator {
        let content_size = mem::size_of::<ConcurrentQueueNode<T>>();
        let content_align = mem::align_of::<ConcurrentQueueNode<T>>();
        let layout = Layout::from_size_align(content_size, content_align).unwrap();

        unsafe { alloc.deallocate(
            NonNull::new(self as *mut _ as *mut u8).unwrap(),
            layout) 
        }
    }

    #[inline]
    fn dummy() -> Self {
        Self {
            data: unsafe { MaybeUninit::uninit().assume_init() },
            next: AtomicUsize::new(0),
        }
    }
}

unsafe impl<T, A: Allocator> Sync for ConcurrentQueue<T, A> {}
unsafe impl<T, A: Allocator> Send for ConcurrentQueue<T, A> {}
