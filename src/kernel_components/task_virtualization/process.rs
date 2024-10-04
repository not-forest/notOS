/// This is an abstraction over the jobs which are done in the OS.

use super::{join_handle::{ThreadOutput, JoinHandle, WriterReference}, PRIORITY_SCHEDULER, ROUND_ROBIN, PROCESS_MANAGEMENT_UNIT};
use super::thread::{Thread, ThreadFn};

use alloc::boxed::Box;
use alloc::vec::Vec;

use core::sync::atomic::{AtomicUsize, Ordering};
use core::ops::{Deref, DerefMut, Drop};
use core::borrow::BorrowMut;
use core::fmt::Debug;
use core::any::Any;
use core::mem;

use crate::{GLOBAL_ALLOCATOR, critical_section};
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
    /// Overall heap memory size of the process. This amount could be changed if the process will
    /// ask for more memory region.
    pub memory_size: usize,
    /// Priority number of the underline process. It should range from 0 to 127, where 0 is the
    /// most significant process.
    pub priority: u8,
    /// Current state of the process.
    pub proc_state: ProcState,
    /// A parent of the current process (if exist).
    pub parent: Option<&'a Process<'a>>,
    /// Universal process id.
    pub(crate) pid: usize,
    /// Process' stack.
    pub(crate) stack: Stack,
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
    ) -> (Self, JoinHandle<T>) where
        F: Fn(&mut Thread) -> T + Send + 'static, T: 'static,
    {
        // Creating an empty process.
        let mut p = Process::new_nomain(stack, memory_size, pid, priority, parent_process);

        // Spawning the main function right away. It is safe because we have just created an empty
        // process with not a single thread within.
        let main_handle = unsafe { p.spawn_main_unchecked(false, main_function) };

        (p, main_handle)
    }

    /// Creates a new process.
    /// 
    /// This function takes another function as a main point of the process. The argument of that function must always be
    /// the thread, that will execute the function in the future. This function must be used if the
    /// handle is not needed from the process.
    pub fn new_void<F, T>(
        stack: Stack,
        memory_size: usize, 
        pid: usize,
        priority: u8,
        parent_process: Option<&'a Process<'a>>,
        main_function: F,
    ) -> Self where
        F: Fn(&mut Thread) -> T + Send + 'static, T: 'static,
    {
        // Creating an empty process.
        let mut p = Process::new_nomain(stack, memory_size, pid, priority, parent_process);

        // Spawning the main function right away. It is safe because we have just created an empty
        // process with not a single thread within.
        unsafe { p.spawn_main_unchecked(true, main_function) };

        p
    }

    /// Creates a new process.
    ///
    /// This function does not require any main function. Putting this process to the PMU will not
    /// add any changes when task switch occur, because there is no main thread with main function.
    /// Yet it will cause some delays in task switching and also take some memory.
    pub fn new_nomain(
        stack: Stack,
        memory_size: usize, 
        pid: usize,
        priority: u8,
        parent_process: Option<&'a Process<'a>>,
    ) -> Self {
        Self {
            stack,
            memory_size,
            priority,
    
            pid,
            proc_state: ProcState::INITIAL,
            parent: parent_process,
   
            threads: ConcurrentList::new(unsafe {&mut GLOBAL_ALLOCATOR }),
        }
    }

    /// Spawns the main thread which will run the provided main function.
    ///
    /// Returns a join handle that will actually not own the real data, therefore it is safe to
    /// drop whenewer needed.
    ///
    /// # Panics
    ///
    /// This function will panic if the main thread already exists within the process.
    pub fn spawn_main<F, T>(&mut self, main_function: F) -> JoinHandle<T> where
        F: Fn(&mut Thread) -> T + Send + 'static, T: 'static 
    {
        assert!(self.threads.len() == 0, "The process already has a main thread.");

        // Afters the checks we are free to spawn the main.
        unsafe{ self.spawn_main_unchecked(false, main_function) } 
    }

    /// Spawns the main thread which will run the provided main function.
    ///
    /// This version of the function is made for processes that return nothing. No 'JoinHandle'
    /// will be returned for this type of functions.
    pub fn spawn_main_void<F, T>(&mut self, main_function: F) where 
        F: Fn(&mut Thread) -> T + Send + 'static, T: 'static 
    {
        assert!(self.threads.len() == 0, "The process already has a main thread.");
    
        unsafe { self.spawn_main_unchecked(true, main_function) };
    }

    /// Spawns a new thread in an existing process
    /// 
    /// Each thread will obtain an individual random id. The thread parameter within the
    /// function is the thread itself that is about to spawn.
    pub fn spawn<F>(&mut self, writer_ref: Option<&'a mut WriterReference>, thread_function: F) where
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

        // Allocating the stack for the thread.
        let thread_stack = self.alloc_stack();

        // Creating the new instance of the thread.
        let mut thread = Thread::new(
            self.pid,
            thread_stack,
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

    /// Allocates the stack for a new thread.
    ///
    /// This function allocates the stack for a new thread request based on the current state of
    /// all existing threads and the function that is being used within the requested thread.
    #[inline]
    pub fn alloc_stack(&mut self) -> Stack {
        // For now allocating 4096 bytes for every thread. TODO! Get rid of this voodoo constant.
        const STARTING_OFFSET: usize = 2 * 4096;
        let mut stack = self.stack.clone();

        // crate::println!("{0:x?}", stack);
        
        stack.top = stack.bottom + STARTING_OFFSET;
        stack.shift_right(STARTING_OFFSET * self.threads.len());

        // crate::println!("{0:x?}", stack);

        stack
    }

    /// Spawns the main thread.
    ///
    /// # Unsafe
    ///
    /// This function will not check the assertion related to main thread. It is also unsafe
    /// because it make the borrow checker to forget about the mutable reference to the process'
    /// output data.
    #[doc(hidden)]
    pub unsafe fn spawn_main_unchecked<F, T>(&mut self, is_void: bool, main_function: F) -> JoinHandle<T> where
        F: Fn(&mut Thread) -> T + Send + 'static, T: 'static 
    {
        // Creating a handle that will not actually own the data.
        let mut main_handle = JoinHandle::new();
        
        /// Based on the function type decides if the thread must return something.
        let unsafe_writer = if is_void {
            None
        } else {
            // Because borrow checker have no idea that main handle does not own the data we must make
            // it forget about it.
            Some(unsafe { (main_handle.writer() as *mut WriterReference).as_mut().unwrap() })
        };

        // Spawning the main thread
        self._inner_spawn(unsafe_writer, main_function); 

        main_handle
    }

    /// Does the spawning with given writer reference and function.
    #[doc(hidden)]
    fn _inner_spawn<F, T>(&mut self, writer_ref: Option<&'a mut WriterReference>, main_function: F) where
        F: Fn(&mut Thread) -> T + Send + 'static, T: 'static,
    {
        // Adding some custom code on top of the recieved function.
        //
        // As a main thread, it must clear up the whole process after itself, which basically means
        // marking the process' state as FINAL.
        critical_section!(|| {
            self.spawn(writer_ref, move |t: &mut Thread| {
                // Getting a process as a resource still must be fair even at this point.
                critical_section!(|| {
                    PROCESS_MANAGEMENT_UNIT.process_list
                        .lock()
                        .get_mut(t.pid)
                        .unwrap().proc_state = ProcState::RUNNING; // Marking the process as running.
                });
                
                let output = main_function(t);

                critical_section!(|| {
                    PROCESS_MANAGEMENT_UNIT.process_list
                        .lock()
                        .get_mut(t.pid)
                        .unwrap().proc_state = ProcState::FINAL; // Marking the process finished.
                });

                Box::new(output)
            });
        });
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
