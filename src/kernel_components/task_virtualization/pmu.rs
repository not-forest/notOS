use crate::kernel_components::arch_x86_64::interrupts::handler_functions::InterruptStackFrame;
/// Module for process management unit.

use crate::kernel_components::structures::{thread_safe::ConcurrentQueue, Single};
use crate::kernel_components::memory::allocators::{GAllocator, GLOBAL_ALLOCATOR};
use crate::kernel_components::sync::Mutex;
use crate::kernel_components::arch_x86_64::interrupts;
use crate::kernel_components::arch_x86_64::controllers::PROGRAMMABLE_INTERRUPT_CONTROLLER;

use crate::single;
use super::{Process, ProcState, Task, Thread, ThreadState, Scheduler, ROUND_ROBIN, PRIORITY_SCHEDULER};

use core::arch::asm;
use core::alloc::{GlobalAlloc, Allocator, Layout};
use core::mem::{self, MaybeUninit, ManuallyDrop};
use core::ptr::{self, NonNull};

/// The main static structure, that contain all processes in the system
single! {
    pub mut PROCESS_MANAGEMENT_UNIT: PMU = PMU::new();
}

/// Process Management Unit.
/// 
/// This unit contain all running process, and provides an easy interface for creating and killing
/// processes.
pub struct PMU<'a> {
    pub process_list: Mutex<PMUList>,
    process_queue: ConcurrentQueue<Process<'a>>,
}

impl<'a> PMU<'a> {
    /// Creates a new instance of PMU.
    /// 
    /// This function will always use only global allocator.
    #[inline]
    pub fn new() -> Self {
        Self {
            process_list: Mutex::new(PMUList::new(unsafe{ &mut GLOBAL_ALLOCATOR })),
            process_queue: ConcurrentQueue::new(unsafe{ &mut GLOBAL_ALLOCATOR }),
        }
    }

    /// Rewrites the current instance of the pmu with the provided new one.
    /// 
    /// # Unsafe
    /// 
    /// This will lead to closing all processes within the
    #[inline]
    pub unsafe fn rewrite(&mut self, pmu: Self) {
        *self = pmu;
    }

    /// Queues the given process.
    /// 
    /// The process first goes to the queue before actually being provided into the list.
    pub fn queue(&mut self, proc: Process<'a>) {
        self.process_queue.enqueue(proc);
    }

    /// Dequeues the process that was first to come.
    ///
    /// The queue within the PMU is a FIFO queue, therefore it is first come first served.
    /// TODO! make privileged process appear faster in the queue.
    pub fn dequeue(&mut self) {
        if let Some(dec) = self.process_queue.dequeue() {
            self.process_list.lock().push_rand(dec);
        }
    }

    /// Removes the process from the list. This function frees the heap memory by deallocating
    /// memory left from the process. It must be called when the process' main thread exited
    /// normally or any other local thread is aborted.
    ///
    /// Works as a wrapper for process_list.lock().remove_proc(pid)
    pub fn remove(&mut self, pid: usize) -> Result<(), ()> {
        self.process_list.lock().remove_proc(pid)
    }

    /// Checks for halted, finished or aborted processes and does the cleaning.
    pub fn cleanup(&mut self) {
        let mut pid = 0; 
        // It will try to clean the processes until it gets one.
        while let Err(()) = self.remove(pid) {pid += 1}
    }
}

/// A small helper list structure, which is not thread safe, so it must be covered in mutex.
pub struct PMUList<A = GAllocator> where A: Allocator + 'static {
    head: usize,
    len: usize,
    alloc: &'static mut A,
}

impl<A: Allocator> PMUList<A> {
    /// Returns the process under the provided pid as a reference.
    #[inline]
    pub fn get(&mut self, pid: usize) -> Option<&Process> {
        let mut next = self.head;
        while let Some(node) = unsafe { (next as *const PMUListNode).as_ref() } {
            if node.process.pid == pid {
                return Some(node.get_proc())
            }
            next = node.next;
        }
        return None
    }

    /// Returns the process under the provided pid as a mutable reference.
    #[inline]
    pub fn get_mut(&mut self, pid: usize) -> Option<&mut Process> {
        let mut next = self.head;
        while let Some(node) = unsafe { (next as *mut PMUListNode).as_mut() } {
            // crate::println!("{:#x?}, {}", node.process, pid);
            if node.process.pid == pid {
                return Some(node.get_proc_mut())
            }
            next = node.next;
        }
        return None
    }

    /// Creates a new PMUList, with no processes.
    #[inline]
    pub fn new(alloc: &'static mut A) -> Self {
        Self {
            head: 0,
            len: 0,
            alloc: alloc,
        }
    }

    /// Pushes the process to the random location.
    /// 
    /// This makes searching through the list fair for new processes that wont be dominated
    /// by those old ones.
    /// 
    /// ## TODO!: ADD RANDOMNESS.
    pub fn push_rand(&mut self, proc: Process) {
        let mut next = self.head;

        let mut new_node = PMUListNode {
            process: proc,
            next: 0,
        };

        let ptr = PMUListNode::node_alloc(new_node, self.alloc);

        if self.head == 0 {
            self.head = ptr as usize;
        }

        while let Some(node) = unsafe { (next as *mut PMUListNode).as_mut() } {
            next = node.next;
        
            if node.next == 0 {
                node.next = ptr as usize;
            }
        }

        self.len.saturating_add(1);
    }

    /// Removes the process from the list based my it's pid.
    ///
    /// # Note
    ///
    /// This function will return Ok(()) if it will manage to find the desired process under the
    /// provided pid, not necessary to remove it.
    pub fn remove_proc(&mut self, pid: usize) -> Result<(), ()> {
        let mut next = self.head;

        while let Some(node) = unsafe { (next as *mut PMUListNode).as_mut() } {
            // Searching for the process.
            if node.process.pid == pid {
                // Doing some changes based on the current state of the process.
                match node.process.proc_state {
                    ProcState::FINAL => {
                        node.node_dealloc(self.alloc);
                        self.len.saturating_sub(1);
                    },
                    _ => (),
                }
                
                return Ok(())
            }

            next = node.next;
        }

        Err(())
    }

    /// Returns the current amount of processes running.
    pub const fn len(&self) -> usize {
        self.len
    }
}

struct PMUListNode<'a> {
    next: usize,
    process: Process<'a>,
}

impl<'a> PMUListNode<'a> {
    /// Returns the item from the node as reference.
    #[inline]
    pub fn get_proc(&self) -> &Process<'a> {
        &self.process
    }

    /// Returns the item from the node as mutable.
    #[inline]
    pub fn get_proc_mut(&mut self) -> &mut Process<'a> {
        &mut self.process
    }

    /// Returns the process within the node with it's ownershipment.
    #[inline]
    pub fn obtain_proc(self) -> Process<'a> {
        self.process
    }

    /// Just allocates the node to some random place on the heap.
    /// 
    /// Returns a pointer to the location.
    fn node_alloc<A>(node: Self, alloc: &mut A) -> *mut Self where A: Allocator {
        let content_size = mem::size_of::<Self>();
        let content_align = mem::align_of::<Self>();
        let layout = Layout::from_size_align(content_size, content_align).unwrap();
        let ptr = unsafe { alloc.allocate(layout) }.unwrap().as_mut_ptr() as *mut Self;

        unsafe {
            ptr.write(node);
        };

        ptr
    }

    // Deallocates the node at given ptr
    fn node_dealloc<A>(&mut self, alloc: &mut A) where A: Allocator {
        let content_size = mem::size_of::<Self>();
        let content_align = mem::align_of::<Self>();
        let layout = Layout::from_size_align(content_size, content_align).unwrap();

        self.process.threads.clear();

        unsafe { alloc.deallocate(
            NonNull::new(self as *mut Self as *mut u8).unwrap(),
            layout) 
        }
    }
}
