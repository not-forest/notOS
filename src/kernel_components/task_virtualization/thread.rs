/// This is a representation of threads. Like in most OS nowadays, threads
/// are the main processing units in the OS.

use core::any::{Any, TypeId};
use core::ptr::NonNull;
use core::fmt::Debug;
use core::cell::RefCell;
use core::sync::atomic::{AtomicUsize, Ordering};

use alloc::boxed::Box;
use alloc::vec::Vec;
use alloc::sync::Arc;

use crate::{critical_section, print};
use crate::kernel_components::arch_x86_64::interrupts::interrupt;
use crate::kernel_components::drivers::timers::{ClockDriver, RealTimeClock};
use crate::kernel_components::drivers::{DRIVER_MANAGER, DriverType, interrupts::InterruptControllerDriver};
use crate::kernel_components::memory::stack_allocator::Stack;
use crate::kernel_components::arch_x86_64::interrupts;
use crate::kernel_components::task_virtualization::{Scheduler, ROUND_ROBIN};
use super::{Process, join_handle::{JoinHandle, HandleStack, WriterReference}, PROCESS_MANAGEMENT_UNIT};

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
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThreadState {
    /// Initiated, yet never ran. This thread's function must be called for the first time.
    INIT,
    // Things that are usually happen most of the time.

    /// The thread is currently doing some tasks.
    RUNNING,
    /// Some thread (or the thread itself) marked the thread to be halted in the next cycle. 
    PREHALT(u8),
    /// The thread is halted until the specified interrupt occurs.
    HALT(u8),
    /// Some other thread made a request to close the current thread. This
    /// behavior can be ignored with PREFINALIGNORE flag.
    PREFINAL,
    /// The thread exited normally. This flag is useful for other threads
    /// that want to communicate with a thread, that is already exited.
    FINAL,

    // Things, which are better not happen a lot.

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
    /// An overall stack allocated for the current thread.
    pub(crate) stack: Stack, 
    /// An instruction pointer of the thread.
    pub(crate) instruction_ptr: AtomicUsize,
    /// A stack pointer of the thread.
    pub(crate) stack_ptr: AtomicUsize,
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
    #[inline]
    pub fn new<F>(process_id: usize, stack: Stack, thread_id: usize, function: F, writer_ref: Option<&'a mut WriterReference>) -> Self where
        F: ThreadFn
    {
        Self {
            pid: process_id,
            tid: thread_id,
            instruction_ptr: AtomicUsize::new(task_switch_call as usize),
            stack_ptr: AtomicUsize::new(stack.top),
            stack: stack,
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
    ///
    /// # Return
    ///
    /// Returns the join handle of the thread. If thread must return any data, it will return data
    /// after calling the join method.
    ///
    /// # Warn
    /// 
    /// This behavior can be recursive of course and could cause some issues.
    #[inline(never)]
    pub fn spawn<F: 'static, T: 'static>(&mut self, thread_function: F) -> JoinHandle<T> where 
        F: (Fn(&mut Thread) -> T) + Send
    {
        // If the function returns () makes thread return nothing.
        let mut handle = JoinHandle::new();

        critical_section!(|| {
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

        handle 
    }

    /// Spawns many thread which perform the same set of intructions.
    ///
    /// Works like spawn function, but instead returns a whole vector of join handles. Provided
    /// function must have signature: |Thread, D, usize| { /* logic */ }, where D is a local
    /// variable and usize is the thread number.
    ///
    /// # Input variables
    ///
    /// User must provide a vector of all local data for each thread. Each value in this vector
    /// will be the next value from the next thread. This function also automatically provides a
    /// integer, which corresponds to number of the thread.
    ///
    /// The amount of threads is decided based on the amount of local variables within the vector.
    /// If threads do not need any local variables, it is ok to just not use them as well as
    /// integers.
    ///
    /// # Return
    ///
    /// Return value is a vector of join handles. Each handle can be manipulated accordingly.
    #[inline(never)]
    pub fn spawn_many<F: 'static, T: 'static, D: 'static>
        (&mut self, locals: Vec<D>, thread_function: F) -> HandleStack<T> where 
            F: (Fn(&mut Thread, D, usize) -> T) + Send + Clone,
            D: Send + Clone,
    {
        let mut v = Vec::new();

        critical_section!(|| {
            for (i, l) in locals.into_iter().enumerate() {
                let fun = thread_function.clone();
                let handle = self.spawn(move |t| { 
                    fun(t, l.clone(), i)
                });

                v.push(handle);
            }
        });

        HandleStack(v)
    }


    /// Marks the thread as PREFINAL, and yields.
    ///
    /// This will allow the scheduler to clear the thread from the process and make some space for
    /// new allocations.
    ///
    /// # Note
    ///
    /// This function only ignores the PREFINALIGNORE flag, since the thread marks itself.
    #[inline(never)]
    pub fn exit(&mut self) {
        // This is always safe as it is a request to ourselves.
        unsafe { self._mark_state(ThreadState::PREFINAL) };
        loop {} // die.
    }

    /// Halts the thread until certain condition is met.
    ///
    /// It will mark the thread as halted and yield the execution. The thread will be
    /// automatically unhalted, after the end of desired interrupt.
    #[inline(always)]
    pub fn halt(&mut self, isr: u8) {
        // This is always safe as it is a halt request.
        unsafe { self._mark_state(ThreadState::PREHALT(isr)) };

        while !self.is_running() { Thread::r#yield() }
    }

    /// Spawns child thread that will only be active when interrupt specified by ISR occurs.
    ///
    /// The thread will run forever as long as the main thread lives, therefore it is useful to
    /// call such interrupt threads from the main process. Such threads may stop themselves by
    /// calling exit() function within the provided closure.
    #[inline]
    pub fn on_isr<F: 'static>(&mut self, isr: u8, f: F) where 
        F: Fn(&mut Thread) + Send
    {
        self.spawn(move |t_isr| loop {
            // Halting the thread until interrupt occurs.
            Thread::halt(t_isr, isr);
            f(t_isr);
        });
    }

    /// Returns true if thread's status is running.
    pub fn is_running(&self) -> bool {
        self.thread_state == ThreadState::RUNNING
    }

    /// Sleeps for the provided amount of milliseconds.
    ///
    /// Until time is not passed, will yield to another thread to do something else. Uses the clock
    /// driver to operate. Will panic if no clock driver is found.
    #[inline(never)]
    pub fn sleep(ms: u32) {
        if let Some(clock) = unsafe{DRIVER_MANAGER.driver::<Box<dyn ClockDriver>>(DriverType::Clock)} {
            let until = clock.now() + ms;

            while let None = clock.dt(until) { Thread::r#yield() }
        } else {
            panic!("No clock driver available for sleep function.");
        }
    }

    /// Function for yielding the thread.
    ///
    /// This function will give up on the processor for the current thread. It works by simply
    /// calling the software interrupt related to task switching. The PIC controller must be
    /// configured right for this function.
    ///
    /// # Panics
    ///
    /// This function will panic only if interrupts are disabled. Yielding the thread while
    /// interrupts are disabled could break the inner logic of the thread's program, therefore 
    /// instead of ignoring the software interrupt completely panic occurs. 
    #[inline(never)]
    pub fn r#yield() {
        let isr = unsafe {
            DRIVER_MANAGER.driver::<Box<dyn InterruptControllerDriver>>(DriverType::Interrupt)
        }
            .map(|int_ctrl| int_ctrl.irq_to_int(0))
            .unwrap_or(0);
            
        interrupt::cause_interrupt(isr);
    }

    /// Halts the thread until certain condition is met.
    ///
    /// It will mark the thread as halted and yield the execution. The thread will be
    /// automatically unhalted, after the end of desired interrupt.
    pub unsafe fn _halt(&mut self, isr: u8) {
        self._mark_state(ThreadState::HALT(isr))
    }

    /// Marks the state as final.
    ///
    /// Used in task switching before deallocating the thread. This flag is also
    /// written to the join handle. It means that the thread is done executing instruction
    /// and exited normally.
    pub unsafe fn _final(&mut self) {
        self._mark_state(ThreadState::FINAL)
    }

    /// Marks the state as running.
    ///
    /// Must be used by task switching handler function, i.e clock interrupts.
    pub unsafe fn _running(&mut self) {
        self._mark_state(ThreadState::RUNNING)
    }

    /// Mutates thread's state together with it's join handle, if exist.
    fn _mark_state(&mut self, s: ThreadState) {
        self.thread_state = s.clone();
        if let Some(o) = &mut self.output {
            o.change_state(s.clone());
        }
    }
}

/// A helper function for calling the new thread, with fast-call calling convention
///
/// This function calls the inner function of the thread and passes the thread itself
/// as a mutable reference argument. This function must be divergent, because we are never
/// calling it ourselves but only jumping to it's address.
///
/// The task switching interrupt must jump to this function with current thread's reference pushed
/// onto the stack.
pub(crate) unsafe fn task_switch_call(t: &mut Thread) -> ! {
    use crate::kernel_components::task_virtualization::{PriorityScheduler, Task, JoinHandle};
    use core::mem;

    let closure = t.fun.as_ref();

    // Running the closure of the thread.
    let output = closure(
        mem::transmute(
            t as *const _ as usize
        )
    );

    critical_section!(|| {
        // Marking the thread as done executing.
        t.thread_state = ThreadState::FINAL;

        // Writing the data to handle and removing the writer.
        if let Some(o) = &mut t.output {
            // Writing data
            o.write(output);
            // Changing the status
            o.change_state(ThreadState::FINAL);
            // Dropping the writer reference and living None on it's place.
            drop(t.output.take());
        }

        // The PC will get here once the task is done. At this moment the task is
        // not needed anymore and can be removed.
        ROUND_ROBIN.delete(
            Task { pid: t.pid, tid: t.tid }
        );

        // Trying to cleanup the process.
        PROCESS_MANAGEMENT_UNIT.remove(t.pid);
    });

    loop {
        interrupt::wait_for_interrupt();
    }
}
