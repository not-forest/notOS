/// A byte representation

use core::{ptr, mem, ops::Deref};

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Ord, Eq)]
#[repr(transparent)]
pub struct Bytes<'a>(&'a [u8]);

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

pub trait AsBytes: Sized {
    fn as_bytes(&self) -> Bytes {
        let ptr = ptr::addr_of!(*self);
        let size = core::mem::size_of::<Self>();
        unsafe { 
            Bytes::new(core::slice::from_raw_parts(ptr.cast(), size)) 
        }
    }
}