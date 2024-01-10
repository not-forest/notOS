/// This is a representation of threads. Like in most OS nowadays, threads
/// are the main processing units in the OS.

use core::{fmt::Debug, marker::Tuple};
use core::ptr::NonNull;
use core::future::Future;
use core::mem;

use alloc::boxed::Box;

use super::{Process, PROCESS_MANAGEMENT_UNIT};

/// A custom trait for thread functions.
/// 
/// Basically they are just normal closures that take the thread
/// as an argument. When implementing the function itself, the
/// input thread must only be represented as a thread that will execute
/// this function.
/// 
/// While the output is void, the function is a closure, that can capture and
/// change it's environment however it wants.
/// 
/// There is a straight rule to have the first parameter as a mutable reference to
/// the thread. There is no need to know about which thread we are referring to, because
/// it is done by the process' spawn method.
pub trait ThreadFn: Fn(&mut Thread) + 'static {}
// This automatically makes all regular Fn closures convertible into ThreadFn closure.
impl<F: Fn(&mut Thread) + Send + 'static> ThreadFn for F {}

/// All the states in which thread can be. Threads may behave differently
/// based on the current state. The state of the process may also change due to 
/// inner threads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadState {
    // Initiated, yet never ran. This thread's function must be called for the
    // first time.
    INIT,
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

/// A struct that acts as a return value from every thread.
/// 
/// The struct will contain the last state of the thread and the returned value from it
/// as a pointer. If the thread was not exited correctly or it did not returned any value, 
/// the value will be None.
#[derive(Debug, Clone, Copy)]
pub struct ThreadOutput<F> {
    thread_state: ThreadState,
    output: Option<F>,
}

/// The main computation units in the OS. Each individual thread lies within the process.
/// 
/// The generic value T is the value which the thread's function must return at the end of
/// execution.
pub struct Thread {
    /// Universal id of the current thread in the process' scope.
    pub tid: usize,
    /// Universal id of the process in which lies the current thread.
    pub pid: usize,
    /// An instruction pointer of the thread.
    pub instruction_ptr: usize,
    /// A stack pointer of the thread.
    pub stack_ptr: usize,
    /// The current state of the thread.
    pub thread_state: ThreadState,
    /// A function that the current thread must perform
    pub fun: Box<dyn ThreadFn>,
}

impl Debug for Thread {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Thread")
            .field("tid", &self.tid)
            .field("pid", &self.pid)
            .field("instruction_pointer", &self.instruction_ptr)
            .field("stack_pointer", &self.stack_ptr)
            .field("thread_state", &self.thread_state)
            .finish()
    }
}

impl Thread {
    /// Creates a new instance of a Thread.
    /// 
    /// Each thread must be a part of some process. All threads must execute
    /// some amount of instructions. Each individual thread must have an individual
    /// tid that cannot collide with other threads within the same process, while can
    /// be the same among other threads.
    pub fn new<F>(process_id: usize, stack_pointer: usize, thread_id: usize, function: F) -> Self where
        F: ThreadFn
    {
        Self {
            pid: process_id,
            tid: thread_id,
            instruction_ptr: 0,
            stack_ptr: stack_pointer,
            thread_state: ThreadState::INIT,
            fun: Box::new(function),
        }
    }

    /// Spawns a new thread within the process of the current thread.
    /// 
    /// This function will not affect the current thread but provide a fast way to call
    /// a spawn method of the current thread's process. This way the thread never dominates
    /// over the others.
    /// 
    /// # Warn
    /// 
    /// This behavior can be recursive of course and could cause some issues.
    pub fn spawn(&self, thread_function: impl ThreadFn + 'static) {
        unsafe {
            let mut list = PROCESS_MANAGEMENT_UNIT.process_list.lock();
            list.get_mut(self.pid)
                .unwrap()
                .spawn(thread_function);
        }
    }
}