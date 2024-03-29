// Module for tags management. It includes tag types, tag ids, tag traits and structs.
// The need for tags arises from the fact that the bootloader and the operating system kernel need to communicate essential details to each other
// to ensure proper initialization and setup of the system. Tags provide a convenient and flexible way to achieve this communication.

use core::ptr::{Pointee, addr_of};
use core::fmt::{Debug, Formatter};
use core::hash::Hash;
use core::marker::PhantomData;
use core::str::Utf8Error;
use proc_macros::Iternum;

// Tag type is a wrapper around u32 for convenient use of binary representation.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct TagTypeId(u32); 

impl TagTypeId {
    #[inline(always)]
    pub const fn new(id: u32) -> Self {
        Self(id)
    }
}

impl Debug for TagTypeId {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        Debug::fmt(&TagType::from(*self), f)
    }
}

// Tag type is a enum, representing the unique identifier for each tag type. 
#[derive(Iternum, Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum TagType {
    End, Cmd, Name, Module, MemInfo, BootDev, Mmap,
    Vbe, FrameBuf, ElfSections, Apm, Efi32, Efi64,
    Smbios, AcpiOld, AcpiNew, Net, EfiMmap, EfiBs, Efi32Ih, Efi64Ih, 
    LoadBase, Additional(u32),
}

impl TagType {
    #[inline(always)]
    pub fn get(&self) -> u32 {
        u32::from(*self)
    }
}

// A trait offers an abstraction over the implementation details of individual tags.
// This trait is required when build custom DSTs.
pub trait TagTrait: Pointee {
    // Id of the specific tag
    const ID: TagType;
    // Returns empty for sized tags. Returns usize for DSTs. This must be implemented for a custom DST
    fn dst_size(tag: &Tag) -> Self::Metadata;
    // Returns the size of the tag.
    fn size(&self) -> usize {
        self.base().size as usize
    }
    // Returns a slice of bytes of the tag and DST.
    fn bytes(&self) -> &[u8] {
        let ptr = addr_of!(*self);
        unsafe { core::slice::from_raw_parts(ptr.cast(), self.size()) }
    }
    // Returns tag as a struct
    fn base(&self) -> &Tag {
        let ptr = addr_of!(*self);
        unsafe { &*ptr.cast::<Tag>() }
    }
    // Returns a ref to a dynamically sized tag.
    unsafe fn from_base_tag<'a>(tag: &Tag) -> &'a Self {
        let ptr = core::ptr::addr_of!(*tag);
        let ptr = core::ptr::from_raw_parts(ptr.cast(), Self::dst_size(tag));
        &*ptr
    }}

// The main tag struct for passing it into MBI
#[derive(Copy, Clone)]
pub struct Tag {
    pub tag_type: TagTypeId,
    pub size: u32,
}

impl Tag {
    // get a tag type from id
    pub fn get_type(&self) -> TagType {
        self.tag_type.into()
    }
    // Casts the base tag to the specific tag type
    pub fn cast_tag<'a, T: TagTrait + ?Sized + 'a>(&'a self) -> &'a T {
        assert_eq!(self.get_type(), T::ID);
        unsafe { TagTrait::from_base_tag(self) }
    }
    // Gets DST as a str slice for some tricky tag
    pub fn get_dst_str_slice(bytes: &[u8]) -> Result<&str, Utf8Error> {
        if bytes.is_empty() {
            return Ok("");
        }

        let str_slice = if bytes.ends_with(&[b'\0']) {
            let str_len = bytes.len() - 1;
            &bytes[0..str_len]
        } else { bytes };
        core::str::from_utf8(str_slice)
    }
}

// The end tag is the last tag in information struct
#[derive(Debug)]
#[repr(C)]
pub struct EndTag {
    pub tag_type: TagTypeId,
    pub size: u32,
}

impl Default for EndTag {
    fn default() -> Self {
        Self {
            tag_type: TagType::End.into(),
            size: 8,
        }
    }
}

impl TagTrait for EndTag {
    const ID: TagType = TagType::End;
    fn dst_size(tag: &Tag) -> Self::Metadata {}
}

// Conversion between types

impl From<u32> for TagTypeId {
    fn from(value: u32) -> Self {
        unsafe { core::mem::transmute(value) }
    }
}

impl From<TagTypeId> for u32 {
    fn from(value: TagTypeId) -> Self {
        value.0 as _
    }
}

impl From<u32> for TagType {
    fn from(value: u32) -> Self {
        match value {
            0..=21 => TagType::get_variant(value as usize),
            _ => TagType::Additional(value),
        }
    }
}

impl From<TagType> for u32 {
    fn from(value: TagType) -> Self {
        match value {
            TagType::Additional(a) => a,
            _ => TagType::get_index(value) as u32
        }
    }
}

impl From<TagTypeId> for TagType {
    fn from(value: TagTypeId) -> Self {
        let temp_u32 = u32::from(value);
        TagType::from(temp_u32)
    }
}

impl From<TagType> for TagTypeId {
    fn from(value: TagType) -> Self {
        let temp_u32 = u32::from(value);
        TagTypeId::from(temp_u32)
    }
}

// A FtL tag iterator.
#[derive(Clone, Debug)]
pub struct TagIter<'a> {
    pub current: *const Tag,
    end_ptr_exclusive: *const u8,
    _mem: PhantomData<&'a ()>,
}

impl<'a> TagIter<'a> {
    /// Creates a new iterator
    pub fn new(mem: &'a [u8]) -> Self {
        assert_eq!(mem.as_ptr().align_offset(8), 0);
        TagIter {
            current: mem.as_ptr().cast(),
            end_ptr_exclusive: unsafe { mem.as_ptr().add(mem.len()) },
            _mem: PhantomData,
        }
    }
}

impl<'a> Iterator for TagIter<'a> {
    type Item = &'a Tag;

    fn next(&mut self) -> Option<&'a Tag> {
        assert!(self.current.cast::<u8>() < self.end_ptr_exclusive);

        let tag = unsafe { &*self.current };
        match tag.get_type() {
            TagType::End => None,
            _ => {
                let ptr_offset = (tag.size as usize + 7) & !7;
                self.current = unsafe { self.current.cast::<u8>().add(ptr_offset).cast::<Tag>() };
                Some(tag)
            }
        }
    }
}
