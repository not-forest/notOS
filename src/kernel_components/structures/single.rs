/// A lazy one-time initialization. This module implements structures
/// that must, by definition, do something once and stay in memory until
/// the whole kernel is shut down.
/// 
/// The `Once` and `Single` structures provide mechanisms for safe, lazy
/// initialization of data that should only be initialized once and then
/// reused across the application.

use crate::kernel_components::memory::allocators::GLOBAL_ALLOCATOR;
use core::alloc::GlobalAlloc;

use crate::kernel_components::sync::Mutex;
use crate::{Vec, single};
use core::fmt::Debug;
use core::sync::atomic::{AtomicBool, Ordering};
use core::cell::{UnsafeCell, Cell};
use core::ops::{Deref, DerefMut};


/// A mechanism for one-time lazy initialization.
///
/// The `Once` structure ensures that a given value is computed at most once
/// and then reused for all subsequent accesses. It uses atomic operations
/// and a mutex for synchronization to guarantee thread-safe initialization.
pub struct Once<T> {
    initialized: AtomicBool,
    data: UnsafeCell<Option<T>>,
}

impl<T: Debug> Debug for Once<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self.get())
    }
}

impl<T> Once<T> {
    /// Creates a new instance of the `Once` structure.
    #[inline(always)]
    pub const fn new() -> Self {
        Self {
            initialized: AtomicBool::new(false),
            data: UnsafeCell::new(None),
        }
    }

    /// Calls the given initialization function if the value has not been initialized yet.
    ///
    /// If the value is already initialized, this method has no effect.
    ///
    /// # Parameters
    ///
    /// - `init`: A closure or function that initializes the data. It is only called
    ///   if the data has not been initialized before.
    ///
    /// # Examples
    ///
    /// ```
    /// use kernel_components::structures::Once;
    ///
    /// let once = Once::new();
    /// once.call(|| 42);
    /// ```
    #[inline(always)]
    pub fn call<F>(&self, init: F) where F: FnOnce() -> T {
        if !self.initialized.load(Ordering::Acquire) {
            unsafe { *self.data.get() = Some(init()) };
            self.initialized.store(true, Ordering::Release);
        }
    }

    /// Calls the given initialization function if the value has not been initialized yet.
    ///
    /// If the value is already initialized, this method has no effect. Arguments can be provided
    /// for the closure.
    #[inline(always)]
    pub fn call_with_args<F, A>(&self, args: A, init: F) where F: FnOnce(A) -> T {
        if !self.initialized.load(Ordering::Acquire) {
            unsafe { *self.data.get() = Some(init(args)) };
            self.initialized.store(true, Ordering::Release);
        }
    }

    /// Forces the initialization of the value and returns an immutable reference to it.
    ///
    /// If the value is not initialized, the given initialization function is called
    /// to initialize it.
    ///
    /// # Parameters
    ///
    /// - `init`: A closure or function that initializes the data if not already initialized.
    ///
    /// # Returns
    ///
    /// A reference to the initialized value.
    ///
    /// # Examples
    ///
    /// ```
    /// use kernel_components::structures::Once;
    ///
    /// let once = Once::new();
    /// let value = once.force(|| 42);
    /// ```
    #[inline(always)]
    pub fn force<F>(&self, init: F) -> &T where F: FnOnce() -> T {
        let value = self.get();
        if value.is_none() {
            self.call(init);
            return self.get().unwrap();
        }
        value.unwrap()
    }

    /// Forces the initialization of the value and returns an immutable reference to it.
    ///
    /// If the value is not initialized, the given initialization function is called
    /// to initialize it.
    ///
    /// # Parameters
    ///
    /// - `init`: A closure or function that initializes the data if not already initialized.
    ///
    /// # Returns
    ///
    /// A mutable reference to the initialized value.
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

/// A structure for lazily initialized singleton data.
///
/// The `Single` structure builds upon the `Once` structure to provide a safe
/// way to create and access a static instance. It allows defining an
/// initialization function that is called to create the initial instance.
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

impl<T: Debug> Debug for Single<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self.data)
    }
}

// Implementations for marker traits. Those are safe, because they use a simple locking mutex.
unsafe impl<T> Sync for Once<T> {}
unsafe impl<T> Send for Once<T> {}
unsafe impl<T, F> Sync for Single<T, F> {}
unsafe impl<T, F> Send for Single<T, F> {}

unsafe impl<A: GlobalAlloc> GlobalAlloc<> for Single<A> {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        self.deref().alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        self.deref().dealloc(ptr, layout);
    }
}

/// A macro to create global 'Once' instance.
/// 
/// Since the static needs to know data types, the output of closure that will be provided must be written
/// from the very start.
/// 
/// # Examples
///
/// ```
/// use crate::global_once;
///
/// // We are creating the static instance of 'Once', knowing that the function that we will
/// // insert must return u8!
/// global_once!(MY_SYSTEM_FUNCTION_THAT_MUST_BE_CALLED_ONLY_ONCE -> u8);
/// 
/// ```
#[macro_export]
macro_rules! global_once {
    ($name:ident -> $type:ty) => {
        static $name: $crate::kernel_components::structures::Once<$type> = $crate::kernel_components::structures::Once::new();
    };
}

/// A macro for creating static instances with lazy initialization.
///
/// The `single!` macro generates static instances of the `Single` structure
/// for each provided name and type, initializing them with the specified
/// initialization function.
/// 
/// # Mutability
/// 
/// If the item will be marked as mutable, every single mutable operation on it
/// must be marked in unsafe block. It is not recommended to use mutable statics,
/// therefore it must only be used if some thread safety are implemented inside the
/// item. If no thread safety is implemented nor the item will never be shared between
/// threads, it is better to wrap the static around mutex or other synchronization primitive.
#[macro_export]
macro_rules! single {
    (
        $(#[$meta:meta])*
        $($name:ident: $type:ty = $init:expr);+ $(;)?
    ) => {
        $(#[$meta])*
        $(
            static $name: $crate::kernel_components::structures::Single<$type> = $crate::kernel_components::structures::Single::new(|| $init);
        )+
    };
    (
        $(#[$meta:meta])*
        $(pub $name:ident: $type:ty = $init:expr);+ $(;)?
    ) => {
        $(#[$meta])*
        $(
            pub static $name: $crate::kernel_components::structures::Single<$type> = $crate::kernel_components::structures::Single::new(|| $init);
        )+
    };
    (
        $(#[$meta:meta])*
        $(mut $name:ident: $type:ty = $init:expr);+ $(;)?
    ) => {
        $(#[$meta])*
        $(
            pub static mut $name: $crate::kernel_components::structures::Single<$type> = $crate::kernel_components::structures::Single::new(|| $init);
        )+
    };
    (
        $(#[$meta:meta])*
        $(pub mut $name:ident: $type:ty = $init:expr);+ $(;)?
    ) => {
        $(#[$meta])*
        $(
            pub static mut $name: $crate::kernel_components::structures::Single<$type> = $crate::kernel_components::structures::Single::new(|| $init);
        )+
    };
    () => ();
}



