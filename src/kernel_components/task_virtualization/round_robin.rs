//! A Round-Robin scheduler implementation module.

use super::{Scheduler, Task, Thread, ThreadFn, Process};
use crate::{GLOBAL_ALLOCATOR, single};
use crate::kernel_components::structures::thread_safe::ConcurrentList;
use core::sync::atomic::{AtomicUsize, Ordering};
use core::any::Any;
use alloc::boxed::Box;

single! {
    pub mut ROUND_ROBIN: RoundRobin = RoundRobin::new();
}

/// A structure that represents the Round-Robin scheduler.
/// 
/// The scheduler returns the next task within the list.
pub struct RoundRobin {
    /// Index of the current running task
    current_task: AtomicUsize,
    /// The list of all tasks
    list: ConcurrentList<Task>,
}

impl RoundRobin {
    /// Creates a new instance of the scheduler.
    pub fn new() -> Self {
        Self { 
            list: ConcurrentList::new(unsafe { &mut GLOBAL_ALLOCATOR }),
            current_task: AtomicUsize::new(0), 
        }
    }

    /// Appends the task based on the provided thread
    pub fn append_thread(&mut self, thread: &Thread) {
        self.list.push(
            Task {
                pid: thread.pid,
                tid: thread.tid,
            }
        );
    }

    /// Appends all the tasks of the given process.
    pub fn append_process(&mut self, proc: &Process) {
        for thread in proc.threads.iter() {
            self.append_thread(thread)
        }
    }
}

impl Scheduler for RoundRobin {
    fn append(&mut self, task: Task) {
        self.list.push(task);
    }

    fn delete(&mut self, task: Task) {
        if let Some(index) = self.list.index_of(task) {
            self.list.remove(index);
        }
    }

    fn current(&mut self) -> Option<&Task> {
        self.list.get(self.current_task.load(Ordering::Relaxed)) 
    }

    fn schedule(&mut self) -> Option<&Task> {
        let mut index = self.current_task.load(Ordering::Acquire) + 1;
        
        if index >= self.list.len() {
            index = 0;
        }

        self.current_task.store(index, Ordering::Release);
        self.list.get(index)
    }

    unsafe fn clear(&mut self) {
        for _ in 0..self.list.len() {
            self.list.pop_front()
        }
    }
}
