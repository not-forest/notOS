//! Lazy one-time initialization structures.
//!
//! Implements structures and single-shot mechanism to initialize or call a method
//! once, with specific lifetime requirements. Those structures can either be used
//! within a certain kernel scope, or be defined static to outlive the entire kernel.
//!
//! ## Usage
//!
//! Structures can be used manually as they are, however the most common use case, is
//! to use the [`single!`] macro to define a lazy-initialized static. Most logical
//! containers within the kernel is initialized using this macro.

use std::cell::{UnsafeCell, Cell};
use std::ops::{Deref, DerefMut};
use std::fmt::Debug;

use osPrimitives::FI_PREVENTION_PATTERN as lazyInit;
use osSync::Mutex;

/// A mechanism for one-time lazy initialization.
///
/// The [`Once`] structure ensures that a given value is computed at most once
/// and then reused for all subsequent accesses. Inner mutex saves the critical section,
/// where data initialization is proceed.
pub struct Once<T> {
    data: UnsafeCell<Option<T>>,
    mutex: Mutex<u8>,
}

impl<T: Debug> Debug for Once<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // Only printing the inner data, since it reflects the `initialized` bool anyway.
        write!(f, "{:?}", self.get())
    }
}

impl<T> Once<T> {
    /// Creates a new instance of the [`Once`] single-shot.
    #[inline]
    pub const fn new() -> Self {
        Self {
            data: UnsafeCell::new(None),
            mutex: Mutex::new(!lazyInit),
        }
    }

    /// Calls the given initialization function if the value has not been initialized yet.
    ///
    /// If the value is already initialized, this method has no effect.
    ///
    /// # Parameters
    ///
    /// * `init`: A closure or function that initializes the data. It is only called
    ///   if the data has not been initialized before.
    ///
    /// # Examples
    ///
    /// ```
    /// let once = Once::new();
    /// once.call(|| 42);
    /// ```
    pub fn call<F>(&self, init: F) where F: FnOnce() -> T {
        if let Ok(flag) = self.mutex.try_lock().as_deref_mut() {
            if *flag != lazyInit {
                unsafe { *self.data.get() = Some(init()) };
                *flag = lazyInit;
            }
        }
    }

    /// Calls the given initialization function if the value has not been initialized yet.
    ///
    /// If the value is already initialized, this method has no effect.
    ///
    /// # Parameters
    ///
    /// * `args`: Arguments to be supplied with the function.
    /// * `init`: A function that initializes the data. It is only called if the data has not 
    /// been initialized before.
    #[inline(always)]
    pub fn call_with_args<F, A>(&self, args: A, init: F) where F: FnOnce(A) -> T {
        self.call(|| init(args)); // This will be inline anyway.
    }

    /// Get reference to underlying data.
    ///
    /// ## Returns
    ///
    /// [`None`] if one-shot function hasn't been called yet. Otherwise, the result of
    /// first [`Once::call`] invocation wrapped in [`Some`] as a reference.
    pub fn get(&self) -> Option<&T> {
        unsafe { &*self.data.get() }.as_ref()
    }

    /// Get reference to underlying data.
    ///
    /// ## Returns
    ///
    /// [`None`] if one-shot function hasn't been called yet. Otherwise, the result of
    /// first [`Once::call`] invocation wrapped in [`Some`] as a mutable reference.
    pub fn get_mut(&self) -> Option<&mut T> {
        unsafe { &mut *self.data.get() }.as_mut()
    }
}

/// A structure for lazily initialized singleton data.
///
/// The [`Single`] structure builds upon the [`Once`] holding the required initialization function.
/// The supplied function is being called upon the first access to the inner data. Structure
/// implements [`Deref`] and [`Derefmut`], therefore it is completely seamless during the usage.
///
/// ## Thread-safety
///
/// Even if both threads tries to get inner data at the same time, and this data is still not
/// initialized, both of them will enter the [`Once::call`] function, where only one of them
/// would actually initialize the data. Other one will just pass through.
pub struct Single<T, F : FnOnce() -> T> {
    once: Once<T>,
    init: Cell<Option<F>>,
}

impl<T, F: FnOnce() -> T> Single<T, F> {
    /// Creates a new instance of [`Single`]
    ///
    /// # Parameters
    ///
    /// * `init`: A closure or function that initializes the data. It is only called
    ///   if the data has not been initialized before.
    #[inline]
    pub const fn new(init: F) -> Self {
        Self {
            once: Once::new(),
            init: Cell::new(Some(init)),
        }
    }
}

impl<T, F: FnOnce() -> T> Deref for Single<T, F> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.once.get().unwrap_or_else(|| {
            self.once.call(self.init.take().unwrap());
            self.once.get().unwrap()    // Mutex would prevent data race.
        })
    }
}

impl<T, F: FnOnce() -> T> DerefMut for Single<T, F> {
    fn deref_mut(&mut self) -> &mut T {
        self.once.get_mut().unwrap_or_else(|| {
            self.once.call(self.init.take().unwrap());
            self.once.get_mut().unwrap() // Mutex would prevent data race.
        })
    }
}

impl<T: Debug, F: FnOnce() -> T> Debug for Single<T, F> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self.once)
    }
}

/// A macro for creating static instances with lazy initialization.
///
/// The [`single!`] macro generates static instances of the [`Single`] structure
/// for each provided name and type, initializing them with the specified
/// initialization function.
/// 
/// ## Mutability
/// 
/// If the item will be marked as mutable, every single mutable operation on it
/// must be marked in unsafe block. It is not recommended to use mutable statics,
/// therefore it must only be used if some thread safety are implemented inside the
/// item. If no thread safety is implemented nor the item will never be shared between
/// threads, it is better to wrap the static around some synchronization primitive.
///
/// #TODO! Add doc examples
#[macro_export]
macro_rules! single {
    (
        $(#[$meta:meta])*
        $($name:ident: $type:ty = $init:expr);+ $(;)?
    ) => {
        $(#[$meta])*
        $(
            static $name: $crate::lazy::Single<$type, fn() -> $type> = $crate::lazy::Single::new(|| $init);
        )+
    };
    (
        $(#[$meta:meta])*
        $(pub $name:ident: $type:ty = $init:expr);+ $(;)?
    ) => {
        $(#[$meta])*
        $(
            pub static $name: $crate::lazy::Single<$type, fn() -> $type> = $crate::lazy::Single::new(|| $init);
        )+
    };
    (
        $(#[$meta:meta])*
        $(mut $name:ident: $type:ty = $init:expr);+ $(;)?
    ) => {
        $(#[$meta])*
        $(
            pub static mut $name: $crate::lazy::Single<$type, fn() -> $type> = $crate::lazy::Single::new(|| $init);
        )+
    };
    (
        $(#[$meta:meta])*
        $(pub mut $name:ident: $type:ty = $init:expr);+ $(;)?
    ) => {
        $(#[$meta])*
        $(
            pub static mut $name: $crate::lazy::Single<$type, fn() -> $type> = $crate::lazy::Single::new(|| $init);
        )+
    };
    () => ();
}
