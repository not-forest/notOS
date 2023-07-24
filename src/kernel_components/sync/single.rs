// Structures for one-time initialization or call. 

use crate::kernel_components::sync::Mutex;
use core::sync::atomic::{AtomicBool, Ordering};
use core::cell::{UnsafeCell, Cell};
use core::ops::{Deref, DerefMut};

pub struct Once<T> {
    initialized: Mutex<AtomicBool>,
    data: UnsafeCell<Option<T>>,
}

impl<T> Once<T> {
    #[inline(always)]
    pub const fn new() -> Self {
        Self {
            initialized: Mutex::new(AtomicBool::new(false)),
            data: UnsafeCell::new(None),
        }
    }

    #[inline(always)]
    pub fn call<F>(&self, init: F) where F: FnOnce() -> T {
        if !self.initialized.lock().load(Ordering::Acquire) {
            unsafe { *self.data.get() = Some(init()) };
            self.initialized.lock().store(true, Ordering::Release);
        }
    }

    #[inline(always)]
    pub fn force<F>(&self, init: F) -> &T where F: FnOnce() -> T {
        let value = self.get();
        if value.is_none() {
            self.call(init);
            return self.get().unwrap();
        }
        value.unwrap()
    }

    #[inline(always)]
    pub fn force_mut<F>(&self, init: F) -> &mut T where F: FnOnce() -> T {
        let value = self.get_mut();
        if value.is_none() {
            self.call(init);
            return self.get_mut().unwrap()
        }
        value.unwrap()
    }

    #[inline(always)]
    pub fn get(&self) -> Option<&T> {
        unsafe { &*self.data.get() }.as_ref()
    }

    #[inline(always)]
    pub fn get_mut(&self) -> Option<&mut T> {
        unsafe { &mut *self.data.get() }.as_mut()
    }
}

pub struct Single<T, F = fn() -> T> {
    data: Once<T>,
    init: Cell<Option<F>>,
}

impl<T, F> Single<T, F> {
    #[inline(always)]
    pub const fn new(init: F) -> Self {
        Self {
            data: Once::new(),
            init: Cell::new(Some(init)),
        }
    }
}

impl<T> Deref for Single<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &*self.data.force(self.init.get().unwrap())
    }
}

impl<T> DerefMut for Single<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut *self.data.force_mut(self.init.get().unwrap())
    }
}

unsafe impl<T> Sync for Once<T> {}
unsafe impl<T> Send for Once<T> {}
unsafe impl<T, F> Sync for Single<T, F> {}
unsafe impl<T, F> Send for Single<T, F> {}

#[macro_export]
macro_rules! single {
    ($($name:ident: $type:ty = $init:expr);+ $(;)?) => {
        $(
            static $name: $crate::kernel_components::sync::Single<$type> = $crate::kernel_components::sync::Single::new(|| $init);
        )+
    };
}



