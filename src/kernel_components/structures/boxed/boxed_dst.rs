// This type behaves similar to the regular
// `Box` type except that it ensure the same layout is used for the (explicit)
// allocation and the (implicit) deallocation of memory.

use crate::Bytes;
use crate::kernel_components::memory::{
    tags::{Tag, TagTrait, TagTypeId},
    allocators::global_alloc::{GAllocator, GLOBAL_ALLOCATOR},
};
use core::alloc::{GlobalAlloc, Layout};
use core::marker::PhantomData;
use core::mem::size_of;
use core::ops::Deref;
use core::ptr::{self, NonNull};

#[derive(Debug, Eq)]
pub struct BoxedDst<T: ?Sized> {
    ptr: core::ptr::NonNull<T>,
    layout: Layout,
    _marker: PhantomData<T>,
}

impl<T: TagTrait<Metadata = usize> + ?Sized> BoxedDst<T> {
    pub fn new(content: &[u8]) -> Self {
        const ALIGN: usize = 4;

        let tag_size = size_of::<TagTypeId>() + size_of::<u32>() + content.len();
        let alloc_size = (tag_size + 7) & !7;
        let layout = Layout::from_size_align(alloc_size, ALIGN).unwrap();
        let ptr = unsafe { GLOBAL_ALLOCATOR.alloc(layout) };
        assert!(!ptr.is_null());

        unsafe {
            // write tag type
            let ptrx = ptr.cast::<TagTypeId>();
            ptrx.write(T::ID.into());

            // write tag size
            let ptrx = ptrx.add(1).cast::<u32>();
            ptrx.write(tag_size as u32);

            // write rest of content
            let ptrx = ptrx.add(1).cast::<u8>();
            let tag_content_slice = core::slice::from_raw_parts_mut(ptrx, content.len());
            for (i, &byte) in content.iter().enumerate() {
                tag_content_slice[i] = byte;
            }
        }

        let base_tag = unsafe { &*ptr.cast::<Tag>() };
        let raw: *mut T = ptr::from_raw_parts_mut(ptr.cast(), T::dst_size(base_tag));

        Self {
            ptr: NonNull::new(raw).unwrap(),
            layout,
            _marker: PhantomData,
        }
    }
}

impl<T: ?Sized> Drop for BoxedDst<T> {
    fn drop(&mut self) {
        unsafe { GLOBAL_ALLOCATOR.dealloc(self.ptr.as_ptr().cast(), self.layout) }
    }
}

impl<T: ?Sized> Deref for BoxedDst<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { self.ptr.as_ref() }
    }
}

impl<T: ?Sized + PartialEq> PartialEq for BoxedDst<T> {
    fn eq(&self, other: &Self) -> bool {
        self.deref().eq(other.deref())
    }
}