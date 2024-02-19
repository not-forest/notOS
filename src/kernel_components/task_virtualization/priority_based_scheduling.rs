/// An implementation of priority based scheduling algorithm.
///
/// This scheduler will only serve high priority processes until they done.
/// TODO! Make the scheduler to change priorities based on halting.

use core::sync::atomic::{AtomicUsize, Ordering};

use crate::kernel_components::structures::thread_safe::ConcurrentList;
use crate::kernel_components::sync::Mutex;
use crate::{single, GLOBAL_ALLOCATOR};

use super::{Scheduler, Task, Thread, ThreadFn, Process, PROCESS_MANAGEMENT_UNIT};

single! {
    pub mut PRIORITY_SCHEDULER: PriorityScheduler = PriorityScheduler::new();
}

/// A priority based scheduler.
///
/// It will always choose the highest priority tasks more, hovewer this will not
/// affect individual threads. If a high priority process have only one thread
/// to execute, it will not schedule until the process is done executing.
///
/// This is a more RTOS styled scheduling algorithm.
pub struct PriorityScheduler {
    /// Index of the current running task
    current_task: AtomicUsize,
    /// The list of all tasks
    list: ConcurrentList<Task>,  
}

impl PriorityScheduler {
    /// Creates a new instance of PriorityScheduler.
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

impl Scheduler for PriorityScheduler {
    fn append(&mut self, task: Task) {
        self.list.push(task); 
    }

    fn delete(&mut self, task: Task) {
        if let Some(index) = self.list.index_of(task) {
            self.list.remove(index);
        }
    }

    fn current(&mut self) -> Option<&Task> {
        self.list.get(self.current_task.load(Ordering::Acquire)) 
    }

    fn schedule(&mut self) -> Option<&Task> {
        let mut index = self.current_task.load(Ordering::Acquire) + 1;

        if index >= self.list.len() {
            index = 0
        }

        if let Some(mut current_task) = self.list.get(index) {
            let mut pri = u8::MAX;

            for (i, task) in self.list.iter().enumerate() {
                unsafe {
                    if let Some(proc) = PROCESS_MANAGEMENT_UNIT.process_list.lock().get(task.pid) {
                        if proc.priority < pri {
                            current_task = task;
                            index = i;
                            pri = proc.priority;
                        } else if proc.priority == 0 {
                            current_task = task;
                            index = i;
                            break
                        }
                    }
                }
            }

            self.current_task.store(index, Ordering::Release);

            return Some(current_task)
        }
 
        None
    }

    unsafe fn clear(&mut self) {
        for _ in 0..self.list.len() {
            self.list.pop_front()
        }
    }
}
