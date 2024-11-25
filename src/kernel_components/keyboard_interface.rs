/// This module implements required interface for commication between user space programs and
/// keyboard driver.

use crate::kernel_components::sync::Mutex;
use crate::single;
use core::{
    ops::{Deref, DerefMut}, 
    sync::atomic::{AtomicU8, Ordering}
};

use super::{arch_x86_64::{controllers::PROGRAMMABLE_INTERRUPT_CONTROLLER, interrupts::INTERRUPT_DESCRIPTOR_TABLE}, task_virtualization::{Thread, ThreadState}};

/// Global static OS char buffer.
single! {
    pub mut OS_CHAR_BUFFER: Mutex<OSCharBuffer> = Mutex::new(OSCharBuffer::new());
}

/// Keyboard Interface.
///
/// Interface for obtaining data from keyboard driver as user-space program. Each program must use
/// this structure to obtain information about the pressed keys. 
#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct KeyboardInterface(u8);

impl KeyboardInterface {
    /// Creates a new instance of KeyboardInterface.
    pub fn new() -> Self {
        Self(
            unsafe{
                OS_CHAR_BUFFER
                    .lock()
                    .keyboard_ptr
                    .load(Ordering::Acquire)
            }
        )
    }

    /// Listens for upcoming keystrokes and does something.
    ///
    /// Creates a new child thread that waits until the keyboard interrupt happens. This
    /// thread will live as long as it's parent, therefore will provide a I/O for single thread.
    ///
    /// The provided function will be called each time the keyboard interrupt is made. The current
    /// thread must be provided in order to spawn a new one, with the function it is going to
    /// execute.
    pub fn on_click<F: 'static, D: 'static>(&mut self, t: &mut Thread, f: F) where
        F: Fn(&mut Thread, Option<&char>) -> D + Send
    {
        // TODO! change when APIC will be implemented.
        let isr = unsafe { &PROGRAMMABLE_INTERRUPT_CONTROLLER }
            .lock()
            .as_mut()
            .map(|pic| pic.master.offset + 1)
            .unwrap_or(0);

        let iface = self.clone();
        t.spawn(move |t| {
            let mut iface = iface.clone();

            loop {
                Thread::halt(t, isr);
                let buf = unsafe { OS_CHAR_BUFFER.deref() }.lock();
                f(t, buf.read(&mut iface));
            }
        });
    }
}

/// A circular buffer used to hold keyboard strokes from the keyboard driver.
///
/// This structure is a bridge for all applications that require user keyboard input.
#[derive(Debug)]
pub struct OSCharBuffer {
    pub keyboard_ptr: AtomicU8,
    buf: [char; 256],
}

impl OSCharBuffer {
    /// Creates a new instance of OSCharBuffer.
    pub fn new() -> Self {
        Self {
            keyboard_ptr: AtomicU8::new(0),
            buf: ['\0'; 256],
        }
    }

    /// Appends one new char to the circular buffer.
    ///
    /// This must only be used by the keyboard interrupt handler that appends a character obtained
    /// from the keyboard driver.
    #[inline]
    pub unsafe fn append(&mut self, c: char) {
        self.buf[self.keyboard_ptr.fetch_add(1, Ordering::SeqCst) as usize] = c;
    }

    /// Reads new lines from the buffer as a single character.
    #[inline]
    pub fn read(&self, inface: &mut KeyboardInterface) -> Option<&char> {
        let ptr = self.keyboard_ptr.load(Ordering::Acquire);
        if inface.0 != ptr {
            let c = &self.buf[inface.0 as usize];
            inface.0 = ptr;
            Some(c)
        } else { None }
    }
}
