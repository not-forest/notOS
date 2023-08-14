// This is an abstraction over the jobs which are done in the OS.

use core::ops::{Deref, DerefMut, Drop};
use super::thread::Thread;
use crate::Vec;
use crate::kernel_components::instructions::RdRand;

/// All states in which the process can be. Processes may behave differently
/// based on the current state.
#[derive(Debug)]
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
pub struct Process<'a> {
    /// Starting memory location of the process.
    memory_loc: *const usize,
    /// End memory location of the process.
    kstack_bottom: *const usize,
    /// Overall memory size of the process.
    memory_size: usize,

    /// Universal process id.
    pid: usize,
    /// Current state of the process.
    proc_state: ProcState,
    /// A parent of the current process (if exist).
    parent: Option<&'a Process<'a>>,

    /// A vector of all threads in the current process.
    threads: Vec<Thread>,
}

impl<'a> Process<'a> {
    /// Creates a new process.
    #[inline(always)]
    pub fn new(memory_location: *const usize, kernel_stack_bottom: *const usize, memory_size: usize, pid: usize, parent_process: Option<&'a Process>, main_function: *mut dyn Fn()) -> Self {
        Self {
            memory_loc: memory_location,
            kstack_bottom: kernel_stack_bottom,
            memory_size,

            pid,
            proc_state: ProcState::INITIAL,
            parent: parent_process,

            threads: Vec::from(
                Thread::new(
                    pid,
                    pid,
                    main_function,
                )
            ),
        }   
    }

    /// Spawns a new thread in an existing process
    #[inline(always)]
    pub fn spawn(&mut self, thread_function: *mut dyn Fn()) {
        let threads_ids: Vec<usize> = self.threads
                                .drain()
                                .map(|item| item.tid)
                                .collect();
        let mut thread_id;
        
        loop {
            thread_id = RdRand::get_u64(RdRand::new().unwrap()).unwrap() as usize;

            if !threads_ids.contains(&thread_id) {
                break;
            }
        }

        self.threads.push(
            Thread::new(
                self.pid,
                thread_id,
                thread_function
            )
        )
    }
} 