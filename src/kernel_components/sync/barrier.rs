/// An implementation for thread barrier.
///
/// Used for multiple threads, which perform same set of instructions or different sets of
/// instruction but must be synchronized in some specific place. Barriers prevents race conditions
/// if used right by the cost of additional time delay.

use crate::kernel_components::task_virtualization::Thread;
use super::Mutex;
use core::sync::atomic::{AtomicUsize, Ordering};

/// A thread barrier.
///
/// Special struct that synchronize chosen threads within one process at some point in the
/// concurrent code. This structure makes no difference except extra delay for a single threaded
/// environment.
#[repr(transparent)]
pub struct Barrier(AtomicUsize);

impl Barrier {
    /// Creates a new instance of 'Barrier'
    ///
    /// Amount of threads must be provided for the barrier to count.
    pub fn new(amount: usize) -> Self {
        Self(AtomicUsize::new(amount))
    }

    /// Flags that the thread has reached a certain point in the code.
    ///
    /// This function synchronizes all marked threads, which shall wait for the others. Each thread
    /// that reaches this function will decrement the barrier counter. When a thread decrements the
    /// pointer once, it will stay in a yield loop, until all threads will enter this function.
    /// 
    /// When all threads have entered this barrier, each thread will enter a spin loop, where it
    /// will wait for others. When each thread have entered the spin loop, they all will continue
    /// together from the barrier point, if in multithreaded invironment.
    pub fn barrier(&self) {
        // Arrived
        if self.0.fetch_sub(1, Ordering::SeqCst) > 1 {
            while self.0.load(Ordering::Acquire) > 0 {
                Thread::r#yield();
            }
        }
    }

    /// Flags that the thread has reached a certain point in the code and executes the provided
    /// closure.
    ///
    /// The closure's logic must be implemented wisely, because thread will just spin in it until
    /// all threads have crossed the barrier.
    pub fn with<F>(&self, f: F) where
        F: Fn()
    {
        // Arrived
        if self.0.fetch_sub(1, Ordering::SeqCst) > 1 {
            while self.0.load(Ordering::Acquire) > 0 {
                f();
            }
        }
    }

    /// Increments the counter.
    ///
    /// This must be not used at all, but sometimes can be helpful.
    unsafe fn append(&mut self, amount: usize) {
        self.0.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |n| Some(n + amount));
    }
}
