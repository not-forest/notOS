use super::join_handle::WriterReference;
/// This is an abstraction over the jobs which are done in the OS.

use super::{join_handle::ThreadOutput, PRIORITY_SCHEDULER, ROUND_ROBIN, PROCESS_MANAGEMENT_UNIT};
use super::thread::{Thread, ThreadFn};

use alloc::boxed::Box;
use alloc::vec::Vec;

use core::borrow::BorrowMut;
use core::ops::{Deref, DerefMut, Drop};
use core::fmt::Debug;
use core::any::Any;
use core::mem;

use crate::GLOBAL_ALLOCATOR;
use crate::kernel_components::arch_x86_64::interrupts::interrupt;
use crate::kernel_components::arch_x86_64::{RdRand, RdSeed};
use crate::kernel_components::memory::stack_allocator::Stack;
use crate::kernel_components::structures::thread_safe::ConcurrentList;

/// All states in which the process can be. Processes may behave differently
/// based on the current state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcState {
    // Things that are usually happen most of the time.
    //
    /// The process is just created with no code and static data loaded into memory. 
    INITIAL,
    /// Has initialized stack, heap and ready to execute, but waiting in a scheduler.
    READY,
    /// Executing some instructions at this moment.
    RUNNING,
    /// Blocked state that halts the process until something happens on background.
    SLEEP,
    /// The process has done executing tasks, but has not yet been cleaned up.
    FINAL,
    
    // Things that are better not happen a lot.
    // 
    /// The process is halted because of some unknown reason.
    BLOCKED,
    /// The process is being killed by some outer activity.
    SLAYED,
    /// One ore more threads panicked inside the process, which made process panic.
    PANICKED,
    /// The process was lost due to some unknown reasons.
    FORBIDDEN,
}

/// The container of all individual threads. The process should contain one or
/// more threads.
#[derive(Debug)]
#[repr(C)]
pub struct Process<'a> {
    /// Universal process id.
    pub(crate) pid: usize,
    /// Overall heap memory size of the process. This amount could be changed if the process will
    /// ask for more memory region.
    pub memory_size: usize,
    /// Priority number of the underline process. It should range from 0 to 127, where 0 is the
    /// most significant process.
    pub priority: u8,
    /// Process' stack.
    pub(crate) stack: Stack,
    /// Current state of the process.
    pub proc_state: ProcState,
    /// A parent of the current process (if exist).
    pub parent: Option<&'a Process<'a>>,
    /// A main handle which is what the process returns after the main thread is done.
    /// pub process_handle: JoinHandle<Box<dyn Any>>,

    /// A list of all threads in the current process.
    pub(crate) threads: ConcurrentList<Thread<'a>>,
}

impl<'a> Process<'a> {
    /// Creates a new process.
    /// 
    /// This function takes another function as a main point of the process. The argument of that function must always be
    /// the thread, that will execute the function in the future.
    pub fn new<F, T>(
        stack: Stack,
        memory_size: usize, 
        pid: usize,
        priority: u8,
        parent_process: Option<&'a Process<'a>>,
        main_function: F,
    ) -> Self where
        F: Fn(&mut Thread) -> T + Send + 'static, T: 'static,
    {
        let mut p = Self {
            stack,
            memory_size,
            priority,
    
            pid,
            proc_state: ProcState::INITIAL,
            parent: parent_process,
    
            threads: ConcurrentList::new(unsafe {&mut GLOBAL_ALLOCATOR }),
        };


        // Adding some custom code on top of the recieved function.
        //
        // As a main thread, it must clear up the whole process after itself, which basically means
        // marking the process' state as FINAL.
        unsafe {
            interrupt::with_int_disabled(|| {   
                p.spawn(None::<&mut WriterReference>, move |t: &mut Thread| {
                    // Getting a process as a resource still must be fair even at this point.
                    interrupt::with_int_disabled(|| {
                        PROCESS_MANAGEMENT_UNIT.process_list
                            .lock()
                            .get_mut(t.pid)
                            .unwrap().proc_state = ProcState::RUNNING; // Marking the process as running.
                    });
                    
                    let output = main_function(t);

                    interrupt::with_int_disabled(|| {
                        PROCESS_MANAGEMENT_UNIT.process_list
                            .lock()
                            .get_mut(t.pid)
                            .unwrap().proc_state = ProcState::FINAL; // Marking the process finished.
                    });

                    Box::new(output)
                });
            }); // The main function must spawn the new thread right away
        }

        p
    }

    /// Spawns a new thread in an existing process
    /// 
    /// Each thread will obtain an individual random id. The thread parameter within the
    /// function is the thread itself that is about to spawn.
    pub fn spawn<F>(&mut self, writer_ref: Option<&'a mut ThreadOutput<Box<dyn Any>>>, thread_function: F) where
        F: ThreadFn + Send
    {
        // Getting the ids of all current threads
        let threads_ids: Vec<usize> = self.threads
                                .iter()
                                .map(|item| item.tid)
                                .collect();        
        let mut thread_id = 0;
       
        // Making sure the id is individual.
        loop {
            if !threads_ids.contains(&thread_id) {
                break;
            }
            thread_id += 1;
        }

        // Creating the new instance of the thread.
        let mut thread = Thread::new(
            self.pid,
            self.stack.top,
            thread_id,
            thread_function,
            writer_ref,
        );

        unsafe {
            // The new thread must be append to the scheduler right away. TODO! Add a more advanced
            // way to append the thread to the scheduler, based on it's status.
            ROUND_ROBIN.append_thread(&thread);
            // Finally push the thread to the list for future contain.
            self.threads.push(thread);
        }
    }

    /// Finds the thread within process' scope by it's tid as a reference.
    pub fn find_thread(&self, tid: usize) -> Option<&Thread> {
        if let Some(thread) = self.threads.iter()
            .find(|t| {t.tid == tid}) {
                Some(thread)
            } else {
                None
            }
    }

    /// Finds the thread within process' score by it's tid as a mutable reference.
    pub fn find_thread_mut(&mut self, tid: usize) -> Option<&mut Thread<'a>> {
        let mut index = 0;
        while let Some(thread) = self.threads.get_mut(index) {
            if thread.tid == tid {
                return Some(thread)
            }
            index += 1;
        }
        None
    }
}

impl<'a> Drop for Process<'a> {
    fn drop(&mut self) {
        self.threads.clear();
    }
}
