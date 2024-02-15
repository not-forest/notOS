use crate::kernel_components::arch_x86_64::interrupts::interrupt;

/// A module for handling the output value from the threads as well as controlling their behavior
/// in terms of synchronisation.

use super::{ThreadState, Thread};

use alloc::{sync::Arc, boxed::Box};
use core::{borrow::Borrow, error::Error, fmt::Display, cell::UnsafeCell, any::Any, marker::PhantomData};

/// A struct that acts as a return value from every thread.
/// 
/// The struct will contain the last state of the thread and the returned value from it
/// as a pointer. If the thread was not exited correctly or it did not returned any value, 
/// the value will be None.
#[derive(Debug)]
pub struct ThreadOutput<T> {
    thread_state: ThreadState,
    output: UnsafeCell<Option<Result<T, ThreadOutputError>>>,
}
 
impl<T> ThreadOutput<T> {
    /// Creates a new empty thread output.
    #[inline(always)]
    fn new() -> Self {
        Self {
            thread_state: ThreadState::INIT,
            output: UnsafeCell::new(None),
        }
    }

    /// Writes the data to the output.
    ///
    /// Must be only used by thread itself after executing it's inner function.
    #[inline]
    pub fn write(&mut self, data: T) {
        self.output.get_mut().replace(Ok(data));
    }

    /// Changes the state data of the thread
    ///
    /// This function must be only used by the thread, so that this status tells the true info.
    #[inline]
    pub fn change_state(&mut self, state: ThreadState) {
        self.thread_state = state;
    } 

    /// Takes the value from the thread output.
    ///
    /// This function will obtain the output data and leave None on it's place. This will prevent
    /// any other threads to obtain the data if the output was somehow copied.  
    #[inline]
    pub fn take(&mut self) -> Result<T, ThreadOutputError> {
        self.output.get_mut().take().unwrap()
    }
}

#[derive(Debug)]
pub enum ThreadOutputError {
    CannotRetrieve
}

impl Error for ThreadOutputError {}

impl Display for ThreadOutputError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        use ThreadOutputError::*;
        match self {
            CannotRetrieve => write!(f, "The thread was unable to write the data into it's output block.")
        }
    }
}

/// A wrapper over thread output that will allow obtaining it after thread exists executing tasks
/// and returns an actual value.
///
/// This structure works as both data transfer mechanism and a synchronisation mechanism that joins
/// individual threads. The real data is being stored as Any trait, so it can be stored in any
/// thread without the need of knowing the data type, since the thread only knows it's data, it is
/// impossible to get the data from other places.
#[derive(Debug)]
pub struct JoinHandle<T> {
    pub(crate) data: Arc<ThreadOutput<Box<dyn Any>>>,
    phantom: PhantomData<T>,
}

impl<T: 'static> JoinHandle<T> {
    /// Creates a new clear instance of 'JoinHandle'.
    ///
    /// The instance itself serves not much purpose without it's corresponding thread, so this
    /// function only makes sense in thread implementations.
    #[inline(always)]
    pub(crate) fn new() -> Self {
        Self {
            data: Arc::new(ThreadOutput::new()),
            phantom: PhantomData
        }
    }

    /// Joins the handle
    ///
    /// Waits until the thread finished doing it's job and returns the threads output value
    /// afterwards. This function consumes the handle and provides error handling when result does
    /// not exist while the thread has in fact exited somehow.
    pub fn join(mut self) -> Result<Box<T>, ThreadOutputError> {
        // Halts until the data exist.
        while !self.exited_normally() {}

        unsafe {
            interrupt::with_int_disabled(|| {
                // Converting Any to datatype.
                if let Some(output) = Arc::get_mut(&mut self.data) {
                    match output.take() {
                        Ok(val) => Ok(val.downcast().ok().unwrap()),
                        Err(err) => Err(err),
                    }
                } else {
                    Err(ThreadOutputError::CannotRetrieve)
                }
            })
        }
    }

    /// Returns a current status of the thread.
    ///
    /// This function will not consume the handle and will copy the value of thread state that is
    /// valid at the moment of the call. If the thread state is not set it can only mean that it is
    /// running.pub fn status(&mut self) -> ThreadState {
    pub fn state(&self) -> ThreadState {
        unsafe {
            interrupt::with_int_disabled(|| {
                self.data.as_ref().thread_state.clone()
            })
        }
    }

    /// Allows to peek and see if the thread has completed it's task and returned
    /// some data. This function will not cause the checker thread to halt in any way.
    pub fn exited_normally(&self) -> bool {
        self.state() == ThreadState::FINAL
    }

    /// Obtains the mutable reference to the handle.
    ///
    /// This mutable reference is only for the writer, therefore until it dies, there will be no other
    /// writes to the thread output. This makes sure that only this writer will be able to write true
    /// value for the output. Only thread itself must be having this value.
    #[inline]
    #[doc(hidden)]
    pub(crate) unsafe fn writer(&mut self) -> &mut WriterReference {
        // panic!("{}", Arc::strong_count(&mut self.data));
        Arc::get_mut(&mut self.data).unwrap()
    }
}

/// A helper type for writer.
pub type WriterReference = ThreadOutput<Box<dyn Any>>; 

