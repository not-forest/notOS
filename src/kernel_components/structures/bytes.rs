/// A byte representation

use alloc::vec::Vec;

use core::{ptr, mem, ops::Deref};

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Ord, Eq)]
#[repr(transparent)]
pub struct Bytes<'a>(pub &'a [u8]);

impl<'a> Bytes<'a> {
    #[inline(always)]
    pub fn new(bytes: &'a [u8]) -> Self {
        Self( bytes )
    }
}

impl<'a> Deref for Bytes<'a> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}

impl<'a> Into<Bytes<'a>> for &'a [u8] {
    fn into(self) -> Bytes<'a> {
        Bytes::new(self)
    }
}

impl<'a> Into<&'a [u8]> for Bytes<'a> {
    fn into(self) -> &'a [u8] {
        self.0.as_ref()
    }
}

/// As bytes transmutation for built in vector struct.
impl<T> AsBytes for Vec<T> {}

/// A trait that provide some DST's to be used within different functions.
/// 
/// Each type that has this trait implemented can be represented as Bytes.
pub trait AsBytes: Sized {
    fn as_bytes(&self) -> Bytes {
        let ptr = ptr::addr_of!(*self);
        let size = core::mem::size_of::<Self>();
        unsafe { 
            Bytes::new(core::slice::from_raw_parts(ptr.cast(), size)) 
        }
    }
}
