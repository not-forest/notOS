/// This is a representation of threads. Like in most OS nowadays, threads
/// are the main processing units in the OS.

use core::any::{Any, TypeId};
use core::ptr::NonNull;
use core::fmt::Debug;
use core::cell::RefCell;

use alloc::boxed::Box;
use alloc::sync::Arc;

use crate::kernel_components::arch_x86_64::interrupts::interrupt;
use crate::kernel_components::arch_x86_64::{
    controllers::PROGRAMMABLE_INTERRUPT_CONTROLLER, interrupts::{self, handler_functions::software::task_switch_call},
};
use super::{Process, join_handle::{JoinHandle, WriterReference}, PROCESS_MANAGEMENT_UNIT};

/// A custom trait for thread functions.
/// 
/// Basically they are just normal closures that take the thread
/// as an argument. When implementing the function itself, the
/// input thread must only be represented as a thread that will execute
/// this function.
/// 
/// While the output is void, the function is a closure, that can capture and/// change it's environment however it wants.
/// 
/// There is a straight rule to have the first parameter as a mutable reference to
/// the thread. There is no need to know about which thread we are referring to, because
/// it is done by the process' spawn method.
pub trait ThreadFn: Fn(&mut Thread) -> Box<dyn Any> + 'static {}
// This automatically makes all regular Fn closures convertible into ThreadFn closure.
impl<F: Fn(&mut Thread) -> Box<dyn Any> + Send + 'static> ThreadFn for F {}

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

/// The main computation units in the OS. Each individual thread lies within the process.
/// 
/// The generic value T is the value which the thread's function must return at the end of
/// execution.
pub struct Thread<'a> {
    /// Universal id of the current thread in the process' scope.
    pub tid: usize,
    /// Universal id of the process in which lies the current thread.
    pub pid: usize,
    /// The current state of the thread.
    pub thread_state: ThreadState,
    /// An instruction pointer of the thread.
    pub(crate) instruction_ptr: usize,
    /// A stack pointer of the thread.
    pub(crate) stack_ptr: usize,
    /// A pointer to output value of the thread
    pub(crate) output: Option<&'a mut WriterReference>,
    /// A function that the current thread must perform
    pub(crate) fun: Box<dyn ThreadFn>,
}

impl Debug for Thread<'_> {
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

impl<'a> Thread<'a> {
    /// Creates a new instance of a Thread.
    /// 
    /// Each thread must be a part of some process. All threads must execute
    /// some amount of instructions. Each individual thread must have an individual
    /// tid that cannot collide with other threads within the same process, while can
    /// be the same among other threads.
    ///
    /// # Join Handle
    ///
    /// If the thread must not return anything the join handle is not needed.
    pub fn new<F>(process_id: usize, stack_pointer: usize, thread_id: usize, function: F, writer_ref: Option<&'a mut WriterReference>) -> Self where
        F: ThreadFn
    {
        Self {
            pid: process_id,
            tid: thread_id,
            instruction_ptr: task_switch_call as usize,
            stack_ptr: stack_pointer,
            thread_state: ThreadState::INIT,
            output: writer_ref,
            fun: Box::new(function),
        }
    }

    /// Spawns a new thread within the process of the current thread.
    /// 
    /// This function will not affect the current thread but provide a fast way to call
    /// a spawn method of the current thread's process. This way the thread never dominates
    /// over the others.
    /// # Return
    ///
    /// Returns the join handle of the thread. If thread must return any data, it will return data
    /// after calling the join method.
    ///
    /// # Warn
    /// 
    /// This behavior can be recursive of course and could cause some issues.
    pub fn spawn<F, T>(&mut self, thread_function: F) -> JoinHandle<T> where 
        F: (Fn(&mut Thread) -> T) + 'static + Send, T: 'static
    {
        // If the function returns () makes thread return nothing.
        let mut handle = JoinHandle::new();

        unsafe {
            interrupts::with_int_disabled(|| {
                PROCESS_MANAGEMENT_UNIT.process_list
                    .lock()
                    .get_mut(self.pid)
                    .unwrap()
                    .spawn(
                        Some (
                            handle.writer()
                        ),
                        
                        move |t| -> Box<dyn Any> {
                            let output = thread_function(t);
    
                            Box::new(output)
                        }
                    );
            });
        }

        handle 
    }

    /// Function for yielding the thread.
    ///
    /// This function will give up on the processor for the current thread. It works by simply
    /// calling the software interrupt related to task switching. The PIC controller must be
    /// configured right for this function.
    ///
    /// # Panics
    ///
    /// This function will panic only if the interrupts are disabled. Yielding the thread while
    /// interrupts are disabled could break the inner logic of the thread, therefore instead of
    /// ignoring the software interrupt completely panic occurs.
    #[inline(always)]
    pub fn r#yield() {
        if interrupt::is_interrupts_enabled() {
            let timer_interrupt_int = PROGRAMMABLE_INTERRUPT_CONTROLLER.lock().get_master_offset();
            crate::println!("{}", timer_interrupt_int);
            interrupt::cause_interrupt(timer_interrupt_int);
        } else {
            panic!("The thread yielded while interrupts are disabled.");
        }
    }
}
