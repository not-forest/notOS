// This is an abstraction over the jobs which are done in the OS.

use core::ops::{Deref, DerefMut, Drop};
use crate::kernel_components::instructions::RdRand;

// All states in which the process can be.
pub enum ProcState {
    // Things that are usually happen most of the time.
    INITIAL, // Just created with no code and static data loaded into memory. 
    READY,  // Has initialized stack, heap and ready to execute, but waiting in a scheduler.
    RUNNING, // Executing some instructions.
    SLEEP, // Blocked state that halts the process until something happens on background.
    FINAL, // The process is exited, but has not yet been cleaned up.
    // Things that are better not happen a lot.
    BLOCKED, // The process is halted because of some unknown reason.
    SLAYED, // The process is being killed by some outer activity.
    PANICKED, // The process made kernel panic, during the execution.
    FORBIDDEN, // The process was lost due to some unknown reasons.
}

pub struct Process<'a> {
    memory_loc: *const usize,
    kstack_bottom: *const usize,
    memory_size: usize,

    pid: usize,
    proc_state: ProcState,
    parent: Option<&'a Process<'a>>,
}

impl<'a> Process<'a> {
    pub fn new(ml: *const usize, kb: *const usize, sz: usize, parent: Option<&'a Process>) -> Self {
        Self {
            memory_loc: ml,
            kstack_bottom: kb,
            memory_size: sz,

            pid: RdRand::new().unwrap().get_u64().unwrap() as usize,
            proc_state: ProcState::INITIAL,
            parent: parent,
        }   
    }
} 