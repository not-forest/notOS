/// Concurrent list data structure implementation, that is thread-safe.
/// 
/// This is a two-sided list that uses lock-free algorithm to insert and delete
/// elements within.

use crate::kernel_components::memory::allocators::GAllocator;
use core::sync::atomic::{AtomicUsize, Ordering, AtomicBool};
use core::alloc::{GlobalAlloc, Allocator, Layout};
use core::marker::PhantomData;
use core::mem::{self, MaybeUninit, ManuallyDrop};
use core::ptr::{self, NonNull};
use core::ops::{Deref, DerefMut, Index};

/// Thread safe concurrent list.
#[derive(Debug)]
pub struct ConcurrentList<T, A = GAllocator> where A: Allocator + 'static {
    head: AtomicUsize,
    tail: AtomicUsize,
    dummy: *mut ConcurrentListNode<T>,
    len: AtomicUsize,
    alloc: &'static mut A,
    _marker: PhantomData<T>,
}

impl<T, A: Allocator> ConcurrentList<T, A> {
    /// Creates a new instance of 'ConcurrentList'.
    /// 
    /// # Dummy
    /// 
    /// This instance will automatically insert a dummy node inside, to make lock-free
    /// algorithm possible. It is necessary to have this dummy node for the algorithm to work,
    /// therefore reading the list's content before adding more data in it, is wrong. 
    pub fn new(alloc: &'static mut A) -> Self {
        let dummy  = ConcurrentListNode::<T>::dummy();
        let ptr = ConcurrentListNode::node_alloc(dummy, alloc);

        Self {
            head: AtomicUsize::new(ptr as usize),
            tail: AtomicUsize::new(ptr as usize),
            dummy: ptr,
            len: AtomicUsize::new(0),
            alloc: alloc,
            _marker: PhantomData,
        }
    }

    /// Works like new and creates a new instance of 'ConcurrentList', but with no dummy node.
    /// 
    /// The content must be provided, as the first element of the list, because it has to be
    /// used instead of the dummy.
    pub fn new_non_dummy(content: T, alloc: &'static mut A) -> Self {
        let node = ConcurrentListNode { 
            data: content,
            exist: AtomicBool::new(true),
            next: AtomicUsize::new(0),
            prev: AtomicUsize::new(0),
        };
        
        let ptr = ConcurrentListNode::node_alloc(node, alloc);

        Self {
            head: AtomicUsize::new(ptr as usize),
            tail: AtomicUsize::new(ptr as usize),
            // There is no dummy, but we must give it some value.
            dummy: ptr,
            alloc: alloc,
            len: AtomicUsize::new(1),
            _marker: PhantomData,
        }
    }

    /// Gets an element on the given index location.
    /// 
    /// # Returns
    /// 
    /// If element do exist on the provided index, returns it as a reference. Returns 'None', if
    /// the index is out of range. It can be out of range from the very start of the function call, or
    /// at the moment when we got to the element index.
    /// 
    /// # Thread safety
    /// 
    /// This read algorithm is lock-free. Every thread will try to iterate over the list
    /// and obtain the most recent data on provided index. If thread will fail to get to the
    /// next pointed node, that would mean that some changes occur in the list, and it should try
    /// again from the start.
    /// 
    /// Not a single can interrupt another reads. The read can fail and start again only when some
    /// other thread modified the state of the list, via some write-like or modify-like operation. 
    pub fn get(&self, index: usize) -> Option<&T> {
        // Try to obtain required node, and then return the value within.
        if let Some(node) = self.inner_get_smart(index) {
            Some(node.obtain())
        } else {
            None
        }
    }

    /// Gets an element on the given index as mutable reference.
    /// 
    /// # Returns
    /// 
    /// If element do exist on the provided index, returns it as a mutable reference. Returns 'None', if
    /// the index is out of range. It can be out of range from the very start of the function call, or
    /// at the moment when we got to the element index.
    /// 
    /// # Thread safety
    /// 
    /// as in the regular get() method, the read algorithm is lock-free. However the later use
    /// of the data must be careful. This function only provides a thread-safe wrapper to get a mutable
    /// reference, however, modifying it's state can cause undefined behavior.
    /// 
    /// If you need to modify the data within the list 'atomically' to other threads. use modify() method
    /// instead. This method is suitable if the element within the list is already some kind of data structure
    /// which is thread-safe via locking or other mechanisms (For example this method could be used for a list
    /// full of AtomicUsize or other atomics).
    pub fn get_mut(&self, index: usize) -> Option<&mut T> {
        // Try to obtain required node, and then return the value within.
        if let Some(node) = self.inner_get_smart(index) {
            Some(node.obtain_mut())
        } else {
            None
        }
    }

    pub fn remove(&mut self, index: usize) {
        'main: loop {
            // If index is out of range, do nothing.
            if index >= self.len() || self.len() == 0 {
                break 'main
            }

            // Trying to obtain the node that lies at given index.
            if let Some(target_node) = self.inner_get_smart(index) {
                // Getting pointers to the neighbors nodes and self.
                let target_node_ptr = target_node as *mut ConcurrentListNode<T> as usize;
                let prev_node_ptr = target_node.prev.load(Ordering::Relaxed);
                let next_node_ptr = target_node.next.load(Ordering::Relaxed);
                 
                // Marking the current node as deleted. It is necessary to not remove the node straightly, because other threads
                // may be reading this thread at that moment.
                target_node.mark_deleted();

                if self.len() != 1 {
                    if index != 0 {
                        let prev_node = if let Some(n) = unsafe {
                            (prev_node_ptr as *mut ConcurrentListNode<T>).as_mut()
                        } { n } else { continue 'main };
                        // Trying to change the pointer to the next node for the previous node.
                        if let Err(_) = prev_node.next.compare_exchange(
                            target_node_ptr, 
                            next_node_ptr, 
                            Ordering::SeqCst, 
                            Ordering::Relaxed,
                        ) {
                            target_node.mark_used();
                            continue 'main
                        }
                    } else {
                        // We do not care if the cas will be able to complete here. If it fails
                        // it can only mean that some other thread did changed the head pointer, to
                        // somewhere else.
                        self.head.compare_exchange(
                            self.head.load(Ordering::Relaxed), 
                            next_node_ptr,
                            Ordering::SeqCst, 
                            Ordering::Relaxed,
                        );
                    }
    
                    if index != self.len() - 1 {
                        let next_node = if let Some(n) = unsafe {
                            (next_node_ptr as *mut ConcurrentListNode<T>).as_mut()
                        } { n } else { continue 'main };
    
                        // Trying to change the pointer to the previous node for the next node.
                        if let Err(_) = next_node.prev.compare_exchange(
                            target_node_ptr, 
                            prev_node_ptr, 
                            Ordering::SeqCst, 
                            Ordering::Relaxed,
                        ) {
                            target_node.mark_used();
                            continue 'main
                        }
                    } else {
                        // We do not care if the cas will be able to complete here. If it fails
                        // it can only mean that some other thread did changed the tail pointer, to
                        // somewhere else.
                        self.tail.compare_exchange(
                            self.tail.load(Ordering::Relaxed), 
                            prev_node_ptr,
                            Ordering::SeqCst, 
                            Ordering::Relaxed,
                        );
                    }
                } else {
                    if self.len() == 0 {
                        unsafe { self.dummy() };
    
                        // We want to change the head and the tail to the new dummy now, because the length
                        // of the list is zero once again. No one would be able to do anything until we have
                        // changed at least one of those values. It is because the head and the tail at that
                        // point points to nodes that do not exist anymore, therefore they will spin in their
                        // reading functions while we do not change those pointers
                        self.head.store(self.dummy as usize, Ordering::Relaxed);
                        self.tail.store(self.dummy as usize, Ordering::Relaxed);
                    }
                }

                self.len.fetch_sub(1, Ordering::SeqCst);

                // At this point we are free to deallocate the node.
                ConcurrentListNode::node_dealloc(target_node_ptr as *mut ConcurrentListNode<T>, self.alloc);

                break 'main
            } else {
                continue 'main
            }
        }
    }

    /// Modifies the element at the given index with a given value.
    /// 
    /// The old content value must be given, to make this function work as a CAS operation,
    /// therefore it will not make any modifications if the old_content doesn't match the data.
    /// Because of that, T must be PartialEq for this specific function.
    /// 
    /// This algorithm creates a new copy of the target node, with the necessary changes,
    /// changes the pointers and marks the previous node as unused. The algorithm is almost
    /// the same as for insert, but instead of creating a new node between the existing ones,
    /// we are just stealing the place for the one of the nodes, and therefore 'changing it's' value
    /// under the hood. For the readers and future writers there will be no change at all.
    /// 
    /// Returns 'None' if the modification operation failed, due to inability to locate
    /// the requested node. Returns Some(&T) otherwise.
    pub fn modify(&mut self, old_content: T, new_content: T, index: usize) -> Option<&T> 
        where T: PartialEq
    {
        let new_content = ManuallyDrop::new(new_content);

        // We are deep copying the provided content.
        //
        // It is necessary, because the insert might not succeed at the first try,
        // therefore we must contain and use the copy of the provided data, because of
        // rusts ownership rules, that will eat the content when we would try to create
        // a new node, without knowing for sure, that we will succeed in changing the pointers
        // of the neighbors nodes.
        //
        // Another way would be either restrict the data type to T: Clone + Copy, or recursively
        // calling this same function over and over, which will put an overhead for the stack.
        let clone = || {
            let mut cloned_content: T = unsafe { MaybeUninit::uninit().assume_init() };
            unsafe {
                ptr::copy_nonoverlapping(
                    &*new_content as *const T,
                    &mut cloned_content as *mut T, 
                    1
                );
            }
            cloned_content
        };

        'main: loop {
            // Trying to find the node, which should be modified.
            if let Some(target_node) = self.inner_get_smart(index) {
                // This is where our clone comes in. Here, if this is not the first iteration,
                // the data would be already moved to the new_node. Our force clone function,
                // makes the data to stay always until we exit the loop.
                let new_content = clone();

                if target_node.data != old_content {
                    break 'main
                }

                // If we managed to get the required node, it is time to create a new one,
                // which we will swap with, the previous one.
                let new_node = ConcurrentListNode {
                    data: new_content,
                    exist: AtomicBool::new(true),
                    prev: AtomicUsize::new(target_node.prev.load(Ordering::Relaxed)),
                    next: AtomicUsize::new(target_node.next.load(Ordering::Relaxed)),
                };

                // Getting the pointers at this moment, because we will loose the new_node afterwards.
                let self_node_ptr = target_node as *mut ConcurrentListNode<T> as usize;
                let prev_node_ptr = new_node.prev.load(Ordering::Relaxed);
                let next_node_ptr = new_node.next.load(Ordering::Relaxed);

                // We must allocate our node before changing the pointers of our neighbors, for a
                // very good reason.
                let ptr = ConcurrentListNode::node_alloc(new_node, self.alloc);

                'inner: loop {
                    // At this point we should mark the target node, as unused. If we fail to do so,
                    // we must retry the whole process again, and usually, it means that this node was 
                    // modified by someone else, or deleted.
                    target_node.mark_deleted();

                    // The order of those CAS operations doesn't really matter, since our
                    // node is already pointing at the right data, and readers will be able to
                    // iterate over our new node already.
                    if index != self.len() - 1 {
                        // Getting the next node as node structure.
                        let next_node = if let Some(n) = unsafe {
                            (next_node_ptr as *mut ConcurrentListNode<T>).as_mut()
                        } { n } else { break 'inner };

                        if let Err(_) = next_node.prev.compare_exchange(
                            self_node_ptr,
                            ptr as usize,
                            Ordering::SeqCst,
                            Ordering::Relaxed,
                        ) {
                            target_node.mark_used();
                            continue 'inner
                        }
                    } else {
                        // It is okay if it fails. It just means that some other thread was first
                        // and we are no longer the tail of the list.
                        self.tail.compare_exchange(
                            self.tail.load(Ordering::Acquire),
                            ptr as usize,
                            Ordering::SeqCst,
                            Ordering::Relaxed,
                        );
                    }

                    if index != 0 {
                        // Getting the prev node as node structure.
                        let prev_node = if let Some(n) = unsafe {
                            (prev_node_ptr as *mut ConcurrentListNode<T>).as_mut()
                        } { n } else { break 'inner };

                        if let Err(_) = prev_node.next.compare_exchange(
                            self_node_ptr,
                            ptr as usize,
                            Ordering::SeqCst,
                            Ordering::Relaxed,
                        ) {
                            target_node.mark_used();
                            continue 'inner
                        }
                    } else {
                        // It is okay if it fails. It just means that some other thread was first
                        // and we are no longer the head of the list.
                        self.head.compare_exchange(
                            self.head.load(Ordering::Acquire),
                            ptr as usize,
                            Ordering::SeqCst,
                            Ordering::Relaxed,
                        );
                    }

                    // It is safe to deallocate the marked node now.
                    ConcurrentListNode::node_dealloc(target_node as *mut ConcurrentListNode<T>, self.alloc);
                    
                    if let Some(node) = unsafe { ptr.as_mut() } {
                        return Some(node)
                    } else {
                        return None
                    }
                }
                // If we managed to get here, that means that we lost a proper info about our
                // neighbors. We must retry the whole process again.
                //
                // There could be many reasons for this to happen, changed state of the previous or
                // next node, or even disappearing.
                //
                // No matter what happened, the previously allocated node is outdated now, so we must
                // deallocate it and try again from the very start.
                ConcurrentListNode::node_dealloc(ptr, self.alloc); // It is okay to just do it like this since we own it.
                continue 'main
            } else {
                break 'main
            }
        }

        return None
    }

    /// Inserts the new element into the provided index.
    pub fn insert(&mut self, content: T, index: usize) {
        self.inner_insert(content, index);
    }

    /// Inserts and automatically returns the reference to the inserted content.
    pub fn insert_return(&mut self, content: T, index: usize) -> Option<&T> {
        if let Some(node) = self.inner_insert(content, index) {
            Some(node)
        } else {
            None
        }
    }

    /// Inserts and automatically returns the mutable reference to the inserted content.
    pub fn insert_return_mut(&mut self, content: T, index: usize) -> Option<&mut T> {
        if let Some(node) = self.inner_insert(content, index) {
            Some(node)
        } else {
            None
        }
    }

    /// Pushes the element to the end of the list.
    /// 
    /// Works the same as insert(last_index), except that you do not need to provide
    /// the index yourself.
    pub fn push(&mut self, content: T) {
        self.insert(content, self.len());
    }

    /// Pushes the element to the start of the list.
    /// 
    /// Works the same as insert(0).
    pub fn push_back(&mut self, content: T) {
        self.insert(content, 0);
    }

    /// Deletes the last element of the list
    /// 
    /// Works the same as remove(last index), except that you do not need to provide
    /// the index yourself.
    pub fn pop(&mut self) {
        self.remove(self.len() - 1);
    }

    /// Deletes the first element of the list
    /// 
    /// Works the same as remove(0).
    pub fn pop_front(&mut self) {
        self.remove(0);
    }

    /// Returns a length of the list.
    pub fn len(&self) -> usize {
        self.len.load(Ordering::Acquire)
    }

    /// Finds the minimal value in the list.
    pub fn min(&self) -> Option<&T> where T: PartialOrd + Ord, {
        self.iter().min()
    }
    
    /// Finds the maximum value in the list
    pub fn max(&self) -> Option<&T> where T: PartialOrd + Ord {
        self.iter().max()
    }

    /// Returns the first element of the list, if it is not empty.
    pub fn head(&self) -> Option<&T> {
        'main: loop {
            // If at some moment of this function execution, this will appear true, return None.
            if self.len() == 0 {
                return None
            }

            // Getting the first element of the list
            let head = self.head.load(Ordering::Acquire);

            // If we got the pointer, try to it's read the corresponding node.
            let head_node = if let Some(n) = unsafe { 
                (head as *mut ConcurrentListNode<T>).as_mut()
            } {
                n
            } else {
                continue 'main
            };

            return Some(head_node.obtain())
        }
    }

    /// Returns the last element of the list, if it is not empty.
    pub fn tail(&self) -> Option<&T> {
        'main: loop {
            // If at some moment of this function execution, this will appear true, return None.
            if self.len() == 0 {
                return None
            }

            // Getting the first element of the list
            let tail = self.tail.load(Ordering::Acquire);

            // If we got the pointer, try to it's read the corresponding node.
            let tail_node = if let Some(n) = unsafe { 
                (tail as *mut ConcurrentListNode<T>).as_mut()
            } {
                n
            } else {
                continue 'main
            };

            return Some(tail_node.obtain())
        }
    }

    /// Finds an index of given item.
    /// 
    /// If the item is not in the list, returns 'None', otherwise, it's index.
    pub fn index_of(&self, content: T) -> Option<usize> where T: PartialEq {
        if let Some((id, _)) = self.iter().enumerate().find(|(_, item)| **item == content) {
            Some(id)
        } else {
            None
        }
    }

    /// Returns an iterator over the list elements.
    pub fn iter(&self) -> ConcurrentListIter<T, A> {
        ConcurrentListIter { list: self, index: 0, length: self.len() }
    }

    /// Removes the dummy value from the list.
    /// 
    /// There is no need to use it manually, because by default, the list does it, if the length
    /// is not zero anymore.
    /// 
    /// # Unsafe
    /// 
    /// This function does no locking nor uses any thread-safe algorithm within. It is an overhead to
    /// provide such functionality just for one dummy value. Inside the insert() and remove() methods,
    /// those type of methods are in a places, where only one thread can be, therefore it is save there.
    /// It is also should not be used when the length of the list is still zero, because it will break the
    /// list algorithm totally.
    pub unsafe fn undummy(&self) {
        ConcurrentListNode::node_dealloc(
            self.dummy.as_mut().unwrap(), self.alloc
        );
    }

    /// Adds the dummy value from the list.
    /// 
    /// There is no need to use it manually, because by default, the list does it, if the length
    /// is zero.
    /// 
    /// # Unsafe
    /// 
    /// This function does no locking nor uses any thread-safe algorithm within. It is an overhead to
    /// provide such functionality just for one dummy value. Inside the insert() and remove() methods,
    /// those type of methods are in a places, where only one thread can be, therefore it is save there.
    /// It is also should not be used when the length of the list is still not zero, because it will allocate
    /// new dummy, while completely ignoring the previous one. This will lead to memory leaks, because we will
    /// never deallocate those unused dummies anymore.
    pub unsafe fn dummy(&mut self) {
        let dummy  = ConcurrentListNode::<T>::dummy();
        let ptr = ConcurrentListNode::node_alloc(dummy, self.alloc);

        self.dummy = ptr;
    }

    /// Returns the immutable reference as a mutable reference.
    /// 
    /// # Unsafe
    /// 
    /// This function completely goes against Rust's ownership and borrowing rules, therefore
    /// this function only exists to satisfy low level kernel behavior, when those rules must be
    /// ignored. It must be used only when you know that you must use it and there is no way out.
    pub unsafe fn return_as_mut(&self) -> &mut Self {
        unsafe { self as *const Self as usize as *mut Self }.as_mut().unwrap()
    }

    /// Returns the current allocator as a mutable reference.
    pub unsafe fn get_alloc(&mut self) -> &mut A {
        &mut self.alloc
    }

    /// Inserts the new element into the provided index.
    /// 
    /// Returns a node as mutable reference.
    fn inner_insert(&mut self, content: T, index: usize) -> Option<&mut ConcurrentListNode<T>> {
        let content = ManuallyDrop::new(content);
        
        // We are deep copying the provided content.
        //
        // It is necessary, because the insert might not succeed at the first try,
        // therefore we must contain and use the copy of the provided data, because of
        // rusts ownership rules, that will eat the content when we would try to create
        // a new node, without knowing for sure, that we will succeed in changing the pointers
        // of the neighbors nodes.
        //
        // Another way would be either restrict the data type to T: Clone + Copy, or recursively
        // calling this same function over and over, which will put an overhead for the stack.
        let clone = || {
            let mut cloned_content: T = unsafe { MaybeUninit::uninit().assume_init() };
            unsafe {
                ptr::copy_nonoverlapping(
                    &*content as *const T,
                    &mut cloned_content as *mut T, 
                    1
                );
            }
            cloned_content
        };
        let mut output;
        
        'main: loop {
            // Try to obtain the node, which would be mutated (pushed forward).
            if let Some(next_node) = self.inner_get_smart(index) {
                // This is where our clone comes in. Here, if this is not the first iteration,
                // the data would be already moved to the new_node. Our force clone function,
                // makes the data to stay always until we exit the loop.
                let new_content = clone();

                // We own the data and the new node, so we can set the pointers right away.
                let new_node = ConcurrentListNode {
                    data: new_content,
                    exist: AtomicBool::new(true),
                    prev: AtomicUsize::new(next_node.prev.load(Ordering::Relaxed)),
                    next: AtomicUsize::new(next_node as *mut ConcurrentListNode<T> as usize),
                };
                // Getting the pointers at this moment, because we will loose the new_node afterwards.
                let prev_node_ptr = new_node.prev.load(Ordering::Relaxed);
                let next_node_ptr = new_node.next.load(Ordering::Relaxed);
                
                // We must allocate our node before changing the pointers of our neighbors, for a
                // very good reason.
                let ptr = ConcurrentListNode::node_alloc(new_node, self.alloc);
                
                // Trying to change the pointers of our neighbors and make them point toward us.
                'inner: loop {
                    // The order of those CAS operations doesn't really matter, since our
                    // node is already pointing at the right data, and readers will be able to
                    // iterate over our new node already.
                    if index != 0 {
                        // Getting the prev node as node structure.
                        let prev_node = if let Some(n) = unsafe {
                            (prev_node_ptr as *mut ConcurrentListNode<T>).as_mut()
                        } { n } else { break 'inner };

                        if let Err(_) = prev_node.next.compare_exchange(
                            next_node_ptr,
                            ptr as usize,
                            Ordering::SeqCst,
                            Ordering::Relaxed,
                        ) {
                            continue 'inner
                        }
                    } else {
                        {
                            // It is okay if it fails. It just means that some other thread was first
                            // and we are no longer the head of the list.
                            self.head.compare_exchange(
                                self.head.load(Ordering::Acquire),
                                ptr as usize,
                                Ordering::SeqCst,
                                Ordering::Relaxed,
                            );
                        }
                    }
                    
                    if index != self.len() - 1 {
                        // Getting the next node as node structure.
                        let next_node = if let Some(n) = unsafe {
                            (next_node_ptr as *mut ConcurrentListNode<T>).as_mut()
                        } { n } else { break 'inner };
                        
                        if let Err(_) = next_node.prev.compare_exchange(
                            prev_node_ptr,
                            ptr as usize,
                            Ordering::SeqCst,
                            Ordering::Relaxed,
                        ) {
                            continue 'inner
                        }
                    }
                    
                    // Returning the allocated node right away.
                    output = unsafe { ptr.as_mut() };
                    
                    break 'main
                }

                // If we managed to get here, that means that we lost a proper info about our
                // neighbors. We must retry the whole process again.
                //
                // There could be many reasons for this to happen, changed state of the previous or
                // next node, or even disappearing.
                //
                // No matter what happened, the previously allocated node is outdated now, so we must
                // deallocate it and try again from the very start.
                ConcurrentListNode::node_dealloc(ptr, self.alloc); // It is okay to just do it like this since we own it.
                continue 'main
                
                // If we unable to get the node that we want to push, it is really an ok situation. Consider
                // the situation when we want to insert new value at the end of the list, there will be nothing here.
                // Or even more unfair, the index became out of range while we tried to do the insert, because the other,
                // threads was faster. It is okay to the overall system, and we must respect it as a natural thing.
                // 
                // Therefore, if it happened, making our node the last of the list
            } else {
                // If it fail, we continue main here because, it means that we are no longer the tail.
                let tail_node = if let Some(n) = unsafe {
                    (self.tail.load(Ordering::Relaxed) as *mut ConcurrentListNode<T>).as_mut()
                } { n } else { continue 'main };
                
                // This is where our clone comes in. Here, if this is not the first iteration,
                // the data would be already moved to the new_node. Our force clone function,
                // makes the data to stay always until we exit the loop.
                let content = clone();

                // We own the data and the new node, so we can set the pointers right away.
                let new_node = ConcurrentListNode {
                    data: content,
                    exist: AtomicBool::new(true),
                    prev: AtomicUsize::new(tail_node as *mut ConcurrentListNode<T> as usize),
                    next: AtomicUsize::new(0),
                };
                
                // We must to allocate our node before changing the pointers of our neighbors, for a
                // very good reason.
                let ptr = ConcurrentListNode::node_alloc(new_node, self.alloc);
                
                'inner: loop {
                    if let Err(_) = tail_node.next.compare_exchange(
                        0,
                        ptr as usize,
                        Ordering::SeqCst,
                        Ordering::Relaxed,
                    ) {
                        break 'inner
                    }
                    
                    // It is okay if it fails. It just means that some other thread was first
                    // and we are no longer the tail of the list.
                    self.tail.compare_exchange(
                        self.tail.load(Ordering::Acquire),
                        ptr as usize,
                        Ordering::SeqCst,
                        Ordering::Relaxed,
                    );
                    
                    // This will be true only for the first insert. We are changing both head and tail to
                    // this node, because we are the only element in the list now. The dummy value will
                    // be deleted, until the length will be zero again.
                    if self.len() == 0 {
                        self.head.compare_exchange(
                            self.head.load(Ordering::Acquire),
                            ptr as usize,
                            Ordering::SeqCst,
                            Ordering::Relaxed,
                        );
                        
                        unsafe { self.undummy() }
                    }

                    // Returning the allocated node right away.
                    output = unsafe { ptr.as_mut() };
                    
                    break 'main
                }
                
                // If we managed to get here, that means that we lost a proper info about our
                // neighbors. We must retry the whole process again.
                //
                // There could be many reasons for this to happen, changed state of the previous or
                // next node, or even disappearing.
                //
                // No matter what happened, the previously allocated node is outdated now, so we must
                // deallocate it and try again from the very start.
                ConcurrentListNode::node_dealloc(ptr, self.alloc); // It is okay to just do it like this since we own it.
                continue 'main
            }
        }
        
        // Increment the length of the list.
        self.len.fetch_add(1, Ordering::SeqCst);
        output
    }
    
    /// Function that chooses between finding algorithm.
    /// 
    /// It will try to find the element that's index is closer to the left side of the list,
    /// it is better to search from the left (head). If we are closer to the end, it is better
    /// to search from the right (tail).
    fn inner_get_smart(&self, index: usize) -> Option<&mut ConcurrentListNode<T>> {
        // It is smaller-or-equal because, if we will find an index, that is in the very middle,
        // the search from left will be faster, because in the search backward algorithm, there is
        // one extra if statement, that will make everything slower by a little.
        if index <= self.len() / 2 {
            self.inner_get(index)
        } else {
            self.inner_get_backward(index)
        }
    }
    
    /// Inner get algorithm.
    ///
    /// It does return the node itself, which must be invisible for the user, therefore
    /// it is private. It also always returns the node as mutable, for further use inside the
    /// write-like functions.
    fn inner_get(&self, index: usize) -> Option<&mut ConcurrentListNode<T>> {
        // The main loop
        'main: loop {
            // If at some moment of this function execution, this will appear true, return None.
            if index >= self.len() {
                return None
            }
            
            // Getting the first element of the list
            let mut head = self.head.load(Ordering::Acquire);
            
            // If we got the pointer, try to it's read the corresponding node.
            let mut next = if let Some(n) = unsafe { 
                (head as *mut ConcurrentListNode<T>).as_mut()
            } {
                n
            } else {
                continue 'main
            };
            
            // Doing the same operations as above, until the required index is obtained.
            for _ in 0..index {
                head = next.next.load(Ordering::Acquire);

                // If the current node that we are checking, is marked as unused, that would
                // mean that we just trapped in between the changing nodes state by some writer
                // thread. Therefore we must retry everything from the start to be sure that
                // we are traveled by a right pointer. Also we should retry is the head somehow,
                // managed to become 0 at this point.
                if head == 0 || !next.exist.load(Ordering::Acquire) {
                    continue 'main
                }

                next = if let Some(n) = unsafe { 
                    (head as *mut ConcurrentListNode<T>).as_mut() 
                } {
                    n
                } else {
                    continue 'main
                };
            }
            
            return Some(next)
        }
    }

    /// Inner get algorithm that goes backwards.
    ///
    /// It does return the node itself, which must be invisible for the user, therefore
    /// it is private. It also always returns the node as mutable, for further use inside the
    /// write-like functions.
    /// 
    /// # Note
    /// 
    /// This algorithm is slower that the forward search by one if statement. However it will be faster,
    /// if we are trying to find the element closer to the end of the list.
    fn inner_get_backward(&self, index: usize) -> Option<&mut ConcurrentListNode<T>> {
        // The main loop
        'main: loop {
            // If at some moment of this function execution, this will appear true, return None.
            if index >= self.len() {
                return None
            }

            // Getting the first element of the list
            let mut tail = self.tail.load(Ordering::Acquire);

            // If we got the pointer, try to it's read the corresponding node.
            let mut prev = if let Some(n) = unsafe { 
                (tail as *mut ConcurrentListNode<T>).as_mut()
            } {
                n
            } else {
                continue 'main
            };

            // Doing the same operations as above, until the required index is obtained.
            for _ in 0..(self.len() - index - 1) {
                tail = prev.prev.load(Ordering::Acquire);

                // If the current node that we are checking, is marked as unused, that would
                // mean that we just trapped in between the changing nodes state by some writer
                // thread. Therefore we must retry everything from the start to be sure that
                // we are traveled by a right pointer. Also we should retry is the tail somehow,
                // managed to become 0 at this point.
                if tail == 0 || !prev.exist.load(Ordering::Acquire) {
                    continue 'main
                }

                // Cloning the data of the last used node.
                let temp_ptr = prev as *mut ConcurrentListNode<T> as usize;
                
                prev = if let Some(n) = unsafe { 
                    (tail as *mut ConcurrentListNode<T>).as_mut()
                } {
                    n
                } else {
                    continue 'main
                };
                
                // Checking if the pointer is still not outdated. This must be done only,
                // for backward search, because the insert() and remove() methods change
                // the next pointer before the prev pointer.
                if prev.next.load(Ordering::Acquire) != temp_ptr && prev.next.load(Ordering::Acquire) != 0 {
                    continue 'main
                }
            }

            return Some(prev)
        }
    }
}

impl<T, A: Allocator> Index<usize> for ConcurrentList<T, A> {
    type Output = T;
    fn index(&self, index: usize) -> &Self::Output {
        loop {
            if let Some(value) = self.get(index) {
                return value
            }
        }
    }
}

impl<T, A: Allocator> Deref for ConcurrentList<T, A> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self[0]
    }
}

impl<T, A: Allocator> Drop for ConcurrentList<T, A> {
    fn drop(&mut self) {
        for _ in 0..self.len() {
            self.pop_front()
        }
    }
}

/// An iterator over the list.
/// 
/// This iterator do not consume the list itself, therefore the values within can be changed,
/// by other threads while we iterate over it.
pub struct ConcurrentListIter<'a, T, A = GAllocator> where A: Allocator + 'static {
    list: &'a ConcurrentList<T, A>,
    index: usize,
    length: usize,
}

impl<'a, T, A: Allocator> Iterator for ConcurrentListIter<'a, T, A> {
    type Item = &'a T;
    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.length {
            let data = self.list.get(self.index);
            self.index += 1;
            data
        } else {
            None
        }
    }
}

/// Single generic node of the concurrent list.
#[derive(Debug)]
struct ConcurrentListNode<T> {
    /// Data that lies within the node.
    data: T,
    /// Flag that marks the node as deleted or not.
    exist: AtomicBool,
    /// Pointer value to the next node.
    next: AtomicUsize,
    /// Pointer value to the previous node.
    prev: AtomicUsize,
}

impl<T> ConcurrentListNode<T> {
    /// Returns the item from the node as ref.
    #[inline]
    pub fn obtain(&self) -> &T {
        self.deref()
    }
    
    /// Returns the item from the node as mutable.
    #[inline]
    pub fn obtain_mut(&mut self) -> &mut T {
        self.deref_mut()
    }
    
    /// Marks the current node as deleted.
    #[inline]
    pub fn mark_deleted(&self) {
        self.exist.compare_exchange(
            true, 
            false, 
            Ordering::SeqCst, 
            Ordering::Relaxed
        );
    }
    
    /// Marks the current node as used.
    #[inline]
    pub fn mark_used(&self) {
        self.exist.compare_exchange(
            false, 
            true, 
            Ordering::SeqCst, 
            Ordering::Relaxed
        );
    }
    
    /// Just allocates the node to some random place on the heap.
    /// 
    /// Returns a pointer to the location.
    #[inline]
    fn node_alloc<A>(node: Self, alloc: &A) -> *mut Self where A: Allocator {
        let content_size = mem::size_of::<ConcurrentListNode<T>>();
        let content_align = mem::align_of::<ConcurrentListNode<T>>();
        let layout = Layout::from_size_align(content_size, content_align).unwrap();
        let ptr = unsafe { alloc.allocate(layout) }.unwrap().as_mut_ptr() as *mut ConcurrentListNode<T>;

        unsafe {
            ptr.write(node);
        };

        ptr
    }

    // Deallocates the node at given ptr
    #[inline]
    fn node_dealloc<A>(ptr: *mut Self, alloc: &A) where A: Allocator {
        let content_size = mem::size_of::<ConcurrentListNode<T>>();
        let content_align = mem::align_of::<ConcurrentListNode<T>>();
        let layout = Layout::from_size_align(content_size, content_align).unwrap();

        unsafe { alloc.deallocate(
            NonNull::new(ptr as *mut u8).unwrap(),
            layout) 
        }
    }

    #[inline]
    fn dummy() -> Self {
        Self {
            data: unsafe { MaybeUninit::uninit().assume_init() },
            exist: AtomicBool::new(true),
            next: AtomicUsize::new(0),
            prev: AtomicUsize::new(0),
        }
    }
}

impl<T> Deref for ConcurrentListNode<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<T> DerefMut for ConcurrentListNode<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.data
    }
}

unsafe impl<T> Sync for ConcurrentList<T> {}
unsafe impl<T> Send for ConcurrentList<T> {}