/// General purpose mutex for the OS.
/// 
/// This mutex uses the regular simple locking algorithm and do not guarantee fairness for each
/// individual thread
/// 
/// # Note
/// 
/// Right now the mutex acts as a spin lock. TODO! make the yield operation possible, after 
/// threads implementation.

use core::fmt::{Debug, Display};
use core::error::Error;
use core::sync::atomic::{AtomicBool, Ordering};
use core::cell::UnsafeCell;
use core::ops::{Drop, Deref, DerefMut};

use crate::kernel_components::arch_x86_64::interrupts::interrupt;
use crate::kernel_components::task_virtualization::Thread;

/// General purpose mutex for the OS.
/// 
/// Can be used to lock some individual structures and guarantee the mutual exclusion for each thread
/// that performs an operation on the requested resource. This mutex implementation yields the CPU if 
/// locked and assures the 
/// 
/// # Poisoning
/// 
/// Poisoning will cause panic of the entire system. (for now).
/// 
/// # Fairness
/// 
/// This mutex algorithm is not fair. Some threads may wait forever, while some others always obtaining
/// the desired resource.
pub struct Mutex<T: ?Sized> {
    status: AtomicBool,
    poisoned: AtomicBool,
    data: UnsafeCell<T>,
}

pub struct MutexGuard<'a, T: 'a + ?Sized>(&'a Mutex<T>);

impl<T> Mutex<T> {
    /// Creates a new instance of the 'Mutex'
    #[inline(always)]
    pub const fn new(data: T) -> Self {
        Self { 
            status: AtomicBool::new(false), 
            poisoned: AtomicBool::new(false),
            data: UnsafeCell::new(data) 
        }
    }

    /// Locks the resource and returns the mutex guard.
    /// 
    /// Other threads that will try to access the desired resource, will be yielded away and the CPU will
    /// obtain some other instructions to follow from the scheduler.
    #[inline(always)]
    pub fn lock(&self) -> MutexGuard<T> {
        match self._inner_lock() {
            Ok(guard) => guard,
            Err(PoisonError) => panic!("{}", PoisonError),
        }
    }

    #[doc(hidden)]
    #[inline(always)]
    fn _inner_lock(&self) -> Result<MutexGuard<T>, PoisonError> {
        while self.status.swap(true, Ordering::Acquire) {
            // Yielding when the lock is taken.
            //Thread::r#yield()
            interrupt::hlt();
        }

        if self.poisoned.load(Ordering::Relaxed) {
            self.status.store(false, Ordering::Release);
            return Err(PoisonError)
        }

        Ok(MutexGuard(self))
    }

    /// Forcefully unlocks the mutex.
    /// 
    /// # Unsafe
    /// 
    /// It is unsafe for a clear reason, but can be useful in some specific situations.
    /// This basically shuts down all the locking prerequisites and makes the resource mutable for any
    /// thread at any time.
    #[inline(always)]
    pub unsafe fn force_unlock(&self) {
        self.status.store(false, Ordering::SeqCst);
    }

    /// Returns the current state of the lock.
    pub fn is_locked(&self) -> bool { self.status.load(Ordering::Relaxed) }
}

impl<'a, T: 'a + ?Sized> Drop for MutexGuard<'a, T> {
    fn drop(&mut self) {
        self.0.status.store(false, Ordering::Release);

        if self.0.poisoned.load(Ordering::Relaxed) {
            panic!("{}", PoisonError);
        }
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

unsafe impl<T> Sync for Mutex<T> {}
unsafe impl<T> Send for Mutex<T> {}

#[derive(Debug)]
pub struct PoisonError;
impl Error for PoisonError {}

impl Display for PoisonError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "The mutex is poisoned and cannot longer be used")
    }
}
