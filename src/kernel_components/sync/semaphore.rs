/// A module that implements two semaphore types.
///
/// The most basic synchronization primitives, which are used within different OS implementation
/// are semaphores. They provide a mutual exclution to some public resource, which has multiple
/// instances, allow to avoid race conditions.

use crate::kernel_components::task_virtualization::Thread;
use super::{Mutex, MutexGuard};
use core::ops::{DerefMut, Deref};
use core::error::Error;
use core::sync::atomic::{Ordering, AtomicUsize};
use core::mem::MaybeUninit;
use core::fmt::Display;
use alloc::boxed::Box;

/// A generic Semaphore
///
/// Blocks the resource for one of the thread, if some other has aquired it first. It is different
/// from a regular Mutex, as it allows a coordination between multiple threads on one resource. It
/// does not own the resource, but guarantees mutual exclution for it.
pub struct Semaphore<T: ?Sized> {
    data: Mutex<Box<T>>,
    value: AtomicUsize,
}

impl<T> Semaphore<T> {
    /// Creates a new instance of BinarySemaphore
    ///
    /// Creates it from the provided data and places it on the heap.
    pub fn new(data: T) -> Self {
        Self {
            data: Mutex::new(Box::new(data)),
            value: AtomicUsize::new(0),
        }
    }

    /// Creates a new instance of BinarySemaphore
    ///
    /// Creates a clear instance of a semaphore, with unitialized data. The data must be carefully
    /// initialized manually by user.
    pub unsafe fn new_ininit() -> Semaphore<MaybeUninit<T>> {
        Semaphore {
            data: Mutex::new(Box::new(MaybeUninit::uninit())),
            value: AtomicUsize::new(0),
        }
    }

    /// Aquires the resource.
    ///
    /// It will only able to obtain it if no other thread is currently working on it. Until
    /// all thread release the resource, it can be obtained again.
    pub fn wait(&self) -> SemaphoreGuard<T> {
        while self.value.load(Ordering::Acquire) > 0 {
            Thread::r#yield();
        }
        self.value.fetch_add(1, Ordering::SeqCst);
        self.data.lock()
    }

    /// Tries to aquire the resource.
    ///
    /// If the condition is not met, will return Err with the current counter value. 
    pub fn try_wait(&self) -> Result<SemaphoreGuard<T>, usize> {
        let counter = self.value.load(Ordering::Acquire); 
        if counter > 0 {
            return Err(counter)
        }

        self.value.fetch_add(1, Ordering::SeqCst);
        Ok(self.data.lock())
    }

    /// Provides a signal to the semaphore and releases the resource.
    pub fn signal(&self, g: SemaphoreGuard<T>) {
        self.value.fetch_sub(1, Ordering::SeqCst);
        drop(g); // Manually dropping the guard.
    }

    /// A proper way to drop the semaphore.
    ///
    /// This function drops the semaphore, but it also returns the last state of the data. 
    pub fn close(self) -> T {
        unsafe { *Mutex::consume(self.data) }
    }
}

/// Custom type that will be returned only from the semaphore.
pub type SemaphoreGuard<'a, T> = MutexGuard<'a, Box<T>>;

unsafe impl<T> Sync for Semaphore<T> {}
unsafe impl<T> Send for Semaphore<T> {}
