// Memory module for memory management. This is the entry point of memory functions and structs. 

use core::mem::size_of;
use crate::MbiLoadError;

use super::{
    tags::{EndTag, TagTrait, TagIter}, 
    memory_map::MemoryMapTag,
    sections::{SectionsTag, SectionIter},
};

// This magic number has 
pub const MAGIC: u32 = 0x36d76289;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(C)]
pub struct BootInfoHeader {
    pub total: u32,
    _reserved: u32,
}

#[repr(C)]
pub struct BootInfo {
    header: BootInfoHeader,
    tags: [u8],
}

impl BootInfoHeader {
    fn new(size: u32) -> Self {
        Self {
            total: size,
            _reserved: 0,
        }
    }
}

impl BootInfo {
    fn end_tag_check(&self) -> bool {
        let end_temp = EndTag::default();
        let self_ptr = unsafe { 
            self.tags
            .as_ptr()
            .sub(size_of::<BootInfoHeader>()) 
        };
        let end_tag_ptr = unsafe {
            self_ptr
            .add(self.header.total as usize)
            .sub(size_of::<EndTag>())    
        };
        let end_tag = unsafe { &*(end_tag_ptr as *const EndTag) };
        end_tag.tag_type == end_temp.tag_type && end_tag.size == end_temp.size
    }
}

// MBI pointer wrapper for boot information
#[repr(transparent)]
pub struct InfoPointer<'a>(&'a BootInfo);

impl<'a> InfoPointer<'a> {
    // Loads the info from the pointer.
    pub unsafe fn load(ptr: *const BootInfoHeader) -> Result<Self, MbiLoadError> {
        if ptr.is_null() || ptr.align_offset(8) != 0 {
            return Err(MbiLoadError::IllegalAddress)
        }

        let mbi = &*ptr;

        if mbi.total == 0 || mbi.total & 0b111 != 0 {
            return Err(MbiLoadError::IllegalTotalSize(mbi.total))
        }

        let slice_size = mbi.total as usize - size_of::<BootInfoHeader>();
        let mbi = &*core::ptr::from_raw_parts::<BootInfo>(ptr.cast(), slice_size);
        
        if !mbi.end_tag_check() {
            return Err(MbiLoadError::NoEndTag);
        }

        Ok(Self(mbi))     
    }

    /// Returns memory map tag. This tag points to memory areas inside the kernel.
    pub fn memory_map_tag(&self) -> Option<&MemoryMapTag> {
        self.get_tag::<MemoryMapTag>()
    }
    
    pub fn elf_sections_tag(&self) -> Option<SectionIter> {
        let tag = self.get_tag::<SectionsTag>();
        tag.map(|t| {
            assert!((t.entry_size * t.shndx) <= t.size);
            t.sections()
        })
    }

    pub fn get_tag<TagT: TagTrait + ?Sized + 'a>(&'a self) -> Option<&'a TagT> {
        self.tags()
            .find(|tag| tag.tag_type == TagT::ID.into())
            .map(|tag| tag.cast_tag::<TagT>())
    }
    
    fn tags(&self) -> TagIter {
        TagIter::new(&self.0.tags)
    }
}


