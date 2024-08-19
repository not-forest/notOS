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

/// A convenient representation of a port, that should be used as a serial-in parallel-out buffer.
///
/// Convenient to use when some large amount of data must be passed via thin port serially. For
/// example passing words or dwords via byte-sized port by performing several reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SipoPort<T, W> where T: Port<T> {
    port: u16,
    access: PortAccessType,
    _phantom: PhantomData<(W, T)>
}

impl<'a, T: Port<T> + Clone> SipoPort<T, T> {
    /// Creates a new instance of SIPO port. 
    #[inline]
    pub fn new(port: u16, access: PortAccessType) -> Self {
        Self {
            port, access,
            _phantom: PhantomData,
        }
    }

    /// Writes the slice of values into the port, if the port is WRITEONLY or READWRITE. The size
    /// of slice defines how much data will be provided at time
    /// 
    /// # Panics
    /// 
    /// Panics if the port is READONLY.
    #[inline]
    pub fn write_many(&self, values: &'a [T]) {
        assert!(self.access != PortAccessType::READONLY, "The port is READONLY.");

        unsafe {
            for val in values {
                T::write(self.port, val.clone());
            }
        }
    }

    /// Reads the values from the port in form of slice, if the port is READONLY or READWRITE. The
    /// size of slice buffer defines how much data should be read in a single chunk.
    /// 
    /// # Panics
    /// 
    /// Panics if the port is WRITEONLY.
    #[inline]
    pub fn read_many(&self, buffer: &'a mut [T]) {
        assert!(self.access != PortAccessType::READONLY, "The port is READONLY.");

        unsafe {
            for buf in buffer {
                *buf = T::read(self.port);
            }
        }
    }
}

impl SipoPort<u8, u16> {
    /// Creates a new instance of SIPO port for shifting words.
    #[inline]
    pub fn new(port: u16, access: PortAccessType) -> Self {
        Self {
            port, access,
            _phantom: PhantomData,
        }
    }

    /// Writes the word value into the port, if the port is WRITEONLY or READWRITE. Word is being
    /// written in little endian format.
    /// 
    /// # Panics
    /// 
    /// Panics if the port is READONLY.
    #[inline]
    pub fn write(&self, value: u16) {
        assert!(self.access != PortAccessType::READONLY, "The port is READONLY.");

        unsafe {
            u8::write(self.port, value as u8);
            u8::write(self.port, (value >> 8) as u8);
        }
    }

    /// Reads the word value from the port, if the port is READONLY or READWRITE. Obtained value is
    /// represented in little endian.
    /// 
    /// # Panics
    /// 
    /// Panics if the port is WRITEONLY.
    #[inline]
    pub fn read(&self) -> u16 {
        assert!(self.access != PortAccessType::WRITEONLY, "The port is WRITEONLY.");

        unsafe {
            u16::from_le_bytes([u8::read(self.port), u8::read(self.port)])
        }
    }
}
