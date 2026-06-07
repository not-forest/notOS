//! General purpose mutex for the OS.
//! 
//! This mutex uses the regular simple locking algorithm and do not guarantee fairness for each
//! individual thread. The thread yields control to the CPU if it has to wait for the mutex.

use std::sync::atomic::{AtomicU8, Ordering};
use std::cell::UnsafeCell;
use std::ops::{Drop, Deref, DerefMut};

use crate::structures::FI_PREVENTION_PATTERN as freeMutex;

type MutexResult<M> = Result<M, ()>;

/// General purpose mutex implementation for the OS.
/// 
/// Can be used to lock some individual structures and guarantee the mutual exclusion for each thread
/// that performs an operation on the requested resource.
/// 
/// # Fairness
/// 
/// This algorithm is not fair. Some threads may wait forever, while some others always obtaining
/// the desired resource.
pub struct Mutex<T: ?Sized> {
    status: AtomicU8,
    data: UnsafeCell<T>,
}

impl<T> Mutex<T> {
    /// Creates a new instance of the 'Mutex'
    #[inline(always)]
    pub const fn new(data: T) -> Self {
        Self { 
            status: AtomicU8::new(freeMutex), 
            data: UnsafeCell::new(data), 
        }
    }

    /// Locks the resource and returns [`MutexGuard`].
    /// 
    /// Other threads that will try to access the desired resource, will be yielded away and the CPU will
    /// obtain some other instructions to follow from the scheduler.
    #[inline]
    pub fn lock(&self) -> MutexGuard<'_, T> {
        if self.__inner_lock().is_err() {
            todo!("Use task switching macro there!");
        }

        MutexGuard(self)
    }

    /// Locks the resource and returns [`MutexGuard`], if not taken already.
    ///
    /// Instead of task switching, returns [`Err`]. Otherwise [`Ok`] with [`MutexGuard`]
    /// is returned.
    pub fn try_lock(&self) -> MutexResult<MutexGuard<'_, T>> {
        self.__inner_lock()
            .map(|_| MutexGuard(self)) 
            .map_err(|_| ())
    } 

    #[doc(hidden)]
    pub fn __inner_lock(&self) -> Result<u8, u8> {
        self.status.compare_exchange(
            freeMutex, 
            !freeMutex, 
            Ordering::Acquire,
            Ordering::Relaxed)
    }

    /// Returns the current state of the lock.
    pub fn is_locked(&self) -> bool { self.status.load(Ordering::Relaxed) != freeMutex }

    /// Consumes the [`Mutex`], obtaining the raw data within.
    ///
    /// After calling this function [`Mutex`] no longer owns the inner data, and it
    /// is moved to one specific thread that called this function.
    pub fn consume(self) -> T {
        let _ = self.lock();
        self.data.into_inner()
    }
}

pub struct MutexGuard<'a, T: 'a + ?Sized>(&'a Mutex<T>);

impl<'a, T: 'a + ?Sized> Drop for MutexGuard<'a, T> {
    fn drop(&mut self) {
        self.0.status.store(freeMutex, Ordering::Release);
    }
}

impl<'a, T> Deref for MutexGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.0.data.get() }
    }
}

impl<'a, T> DerefMut for MutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.0.data.get() }
    }
}
