/// The main crate for schedulers.

use super::Thread;

/// A trait that represents the scheduler that can be used for task scheduling.
/// 
/// Each custom scheduler must have this trait to work properly with timer interrupt.
/// 
/// # Tasks
/// 
/// Tasks are addresses of the threads that should be scheduled. The processes only matter
/// as the structures that create and provide different threads to work in it's scope, when
/// the threads are doing all the work. The combination of both are tasks.
pub trait Scheduler {
    /// Appends the new task to the scheduler.
    /// 
    /// The function takes the current running task and adds this info to the list.
    fn append(&mut self, task: Task);

    /// Deletes the task from the scheduler's list.
    fn delete(&mut self, task: Task);

    /// The main function for scheduling.
    /// 
    /// This function is being called within the timer interrupt, to perform task switching.
    /// It must return the next task, based on the inner algorithm.
    fn schedule(&mut self) -> Option<&Task>;

    /// Clears the process queue if the scheduler use some list-like structure to contain current
    /// running programs.
    /// 
    /// # Unsafe
    /// 
    /// This function must be used when some fatal error, that can only be fixed via cleaning the
    /// entire process space.
    unsafe fn clear(&mut self);
}

/// A struct that provides all needed info about the current running task
/// 
/// This struct contains the pid and tid pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(C)]
pub struct Task {
    pub(crate) pid: usize,
    pub(crate) tid: usize,
}

impl Task {
    /// Creates a new task
    /// 
    /// # Unsafe
    /// 
    /// This function is unsafe, as the task must be correct for the scheduler to work properly.
    pub const unsafe fn new(pid: usize, tid: usize) -> Self {
        Self {
            pid, tid
        }
    }
}
