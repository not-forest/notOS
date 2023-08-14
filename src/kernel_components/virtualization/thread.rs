/// This is a representation of threads. Like in most OS nowadays, threads
/// are the main processing units in this OS.

use core::ptr::NonNull;

/// All the states in which thread can be. Threads may behave differently
/// based on the current state. The state of the process may also change due to 
/// inner threads.
#[derive(Debug)]
pub enum ThreadState {
    // Things that are usually happen most of the time.
    //
    /// The thread is currently doing some tasks.
    RUNNING,
    /// Some other thread made a request to close the current thread. This
    /// behavior can be ignored with PREFINALIGNORE flag.
    PREFINAL,
    /// The thread exited normally. This flag is useful for other threads
    /// that want to communicate with a thread, that is already exited.
    FINAL,

    // Things that are better not happen a lot.
    // 
    /// The thread has panicked. This will cause the whole process to panic. 
    PANICKED,
    /// This flag ignores the PREFINAL flag. With it, the thread can only exit manually.
    PREFINALIGNORE,

    // Both PREFINAL and PREFINALIGNORE act like a RUNNING flag with an extra information
    // for those threads who want to close current thread.
}

/// The main computation units in the OS. Each individual thread lies within the process.
#[derive(Debug)]
pub struct Thread {
    /// Universal id of the current thread in the process' scope.
    pub(crate) tid: usize,
    /// Universal id of the process in which lies the current thread.
    pid: usize,
    /// The current state of the thread.
    thread_state: ThreadState,

    /// Pointer to a function which should be executed by this thread.
    fun: NonNull<dyn Fn()>,
}

impl Thread {
    pub fn new(process_id: usize, thread_id: usize, function: *mut dyn Fn()) -> Self {
        Self {
            pid: process_id,
            tid: thread_id,
            thread_state: ThreadState::RUNNING,

            fun: NonNull::new(function).unwrap(),
        }
    }
}

impl PartialEq<usize> for Thread {
    fn eq(&self, other: &usize) -> bool {
        self.tid == *other
    }
}