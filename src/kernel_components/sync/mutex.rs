// A basic mutex implementation for the OS with a spin loop.

use core::sync::atomic::{AtomicBool, Ordering};
use core::cell::UnsafeCell;
use core::ops::{Drop, Deref, DerefMut};
use crate::kernel_components::error::PoisonError;

pub struct Mutex<T: ?Sized> {
    status: AtomicBool,
    poisoned: AtomicBool,
    data: UnsafeCell<T>,
}

pub struct MutexGuard<'a, T: 'a + ?Sized>(&'a Mutex<T>);

impl<T> Mutex<T> {
    #[inline(always)]
    pub fn new(data: T) -> Self {
        Self { 
            status: AtomicBool::new(false), 
            poisoned: AtomicBool::new(false),
            data: UnsafeCell::new(data) 
        }
    }

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
            crate::kernel_components::instructions::interrupt::hlt();
        }

        if self.poisoned.load(Ordering::Relaxed) {
            self.status.store(false, Ordering::Release);
            return Err(PoisonError)
        }

        Ok(MutexGuard(self))
    }

    #[inline(always)]
    pub unsafe fn force_unlock(&self) {
        self.status.store(false, Ordering::SeqCst);
    }

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
