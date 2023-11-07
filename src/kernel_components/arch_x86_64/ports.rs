/// A module for managing i/o port connections.

use core::{arch::asm, marker::PhantomData};

/// Privilege levels of the ports, that provide the ability to read, write and both.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortAccessType {
    READONLY, WRITEONLY, READWRITE,
}

/// A trait that represent a single generic port.
pub trait Port<T> {
    /// Reads the value from the provided port.
    unsafe fn read(port: u16) -> T;
    /// Writes the value into the provided port.
    unsafe fn write(port: u16, value: T);
}

#[doc(hidden)]
macro_rules! implement_port {
    ($numeric_format:ty, $reg:tt) => {
        impl Port<$numeric_format> for $numeric_format {
            unsafe fn read(port: u16) -> $numeric_format {
                let value: $numeric_format;

                unsafe {
                    asm!(concat!("in ", $reg, ", dx"), out($reg) value, in("dx") port, options(nomem, nostack, preserves_flags));
                }

                value
            }

            unsafe fn write(port: u16, value: $numeric_format) {
                unsafe {
                    asm!(concat!("out dx, ", $reg), in("dx") port, in($reg) value, options(nomem, nostack, preserves_flags));
                }
            }
        }
    };
    () => {};
}

implement_port!(u8, "al");
implement_port!(u16, "ax");
implement_port!(u32, "eax");

/// A generic port representation as a struct.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericPort<T> where T: Port<T> {
    port: u16,
    access: PortAccessType,
    _phantom: PhantomData<T>,
}

impl<T: Port<T>> GenericPort<T> {
    /// Creates a new instance of a port.
    /// 
    /// The access type must be chosen with caution, because it can cause undefined behavior on
    /// later use.
    #[inline]
    pub const fn new(port: u16, access: PortAccessType) -> Self {
        Self {
            port, access, _phantom: PhantomData,
        }
    }

    /// Writes the value into the port, if the port is WRITEONLY or READWRITE.
    /// 
    /// # Panics
    /// 
    /// Panics if the port is READONLY.
    #[inline]
    pub fn write(&self, value: T) {
        assert!(self.access != PortAccessType::READONLY, "The port is READONLY.");

        unsafe {
            T::write(self.port, value);
        }
    }

    /// Reads the value from the port, if the port is READONLY or READWRITE.
    /// 
    /// # Panics
    /// 
    /// Panics if the port is WRITEONLY.
    #[inline]
    pub fn read(&self) -> T {
        assert!(self.access != PortAccessType::WRITEONLY, "The port is WRITEONLY.");

        unsafe {
            T::read(self.port)
        }
    }
} 



