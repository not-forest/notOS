/// A special tag that contain section information about the .elf executable
/// which is the kernel itself.

use core::fmt::{Debug, Formatter};
use core::marker::PhantomData;
use core::mem;
use core::str::Utf8Error;
use core::ops::{Deref, DerefMut};
use core::ptr::Pointee;

use proc_macros::Iternum;
use crate::{Vec, AsBytes};
use crate::kernel_components::structures::{
    IternumTrait,
    boxed::BoxedDst,
};

use super::tags::{TagTrait, TagType, TagTypeId, Tag};

const METADATA_SIZE: usize = mem::size_of::<TagTypeId>() + 4 * mem::size_of::<u32>();

/// Special tag that contain the section header from an ELF binary.
#[derive(PartialEq, Eq)]
#[repr(C)]
pub struct SectionsTag {
    tag_type: TagTypeId,
    pub(crate) size: u32,
    num_of_sections: u32,
    pub(crate) entry_size: u32,
    pub(crate) shndx: u32,
    sections: [u8],
}

impl SectionsTag {
    /// Creates a new elf section tag.
    pub(crate) fn new(
        num_of_sections: u32,
        entry_size: u32,
        shndx: u32,
        sections: &[u8],
    ) -> BoxedDst<Self> {
        let mut bytes = Vec::from_array(&[
            num_of_sections.as_bytes(),
            entry_size.as_bytes(),
            shndx.as_bytes()
        ]);
        bytes.push(sections.into());

        BoxedDst::new(bytes.as_bytes().into())
    }

    /// Returns an iterator of loaded ELF sections.
    pub(crate) fn sections(&self) -> SectionIter {
        let string_section_offset = (self.shndx * self.entry_size) as isize;
        let string_section_ptr = unsafe {
            self.first_section().offset(string_section_offset) as *const _
        };
        SectionIter {
            current_section: self.first_section(),
            remaining_sections: self.num_of_sections,
            entry_size: self.entry_size,
            string_section: string_section_ptr,
        }
    }
    
    fn first_section(&self) -> *const u8 {
        &(self.sections[0]) as *const _
    }
}

impl TagTrait for SectionsTag {
    const ID: TagType = TagType::ElfSections;

    fn dst_size(tag: &Tag) -> Self::Metadata {
        assert!(tag.size as usize >= METADATA_SIZE);
        tag.size as usize - METADATA_SIZE
    }
}

impl Debug for SectionsTag {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SectionTag")
        .field("tag_type", &{ self.tag_type })
        .field("size", &{ self.size })
        .field("num_of_sections", &{ self.num_of_sections })
        .field("entry_size", &{ self.entry_size })
        .field("shndx", &{ self.shndx })
        .field("sections", &self.sections())
        .finish()
    }
}

/// A single generic ELF section
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ElfSection {
    pub inner: *const u8,
    string_section: *const u8,
    entry_size: u32,
}

impl ElfSection {
    /// Returns a section type.
    pub fn section_type(&self) -> ElfSectionType {
        match self.get().typ() {
            i@ 0..=11 => ElfSectionType::get_variant(i as usize),
            0x6000_0000..=0x6FFF_FFFF => ElfSectionType::EnvironmentSpecific,
            0x7000_0000..=0x7FFF_FFFF => ElfSectionType::ProcessorSpecific,
            u => ElfSectionType::Unused,
        }
    }

    pub fn get(&self) -> &dyn ElfSectionInner {
        match self.entry_size {
            40 => unsafe { &*(self.inner as *const ElfSectionInner32) },
            64 => unsafe { &*(self.inner as *const ElfSectionInner64) },
            s => panic!("Unexpected entry size: {}", s),
        }
    }

    /// Get the "raw" section type as a `u32`
    pub fn section_type_raw(&self) -> u32 {
        self.get().typ()
    }

    /// Read the name of the section.
    pub fn name(&self) -> Result<&str, Utf8Error> {
        use core::{slice, str};

        let name_ptr = unsafe { self.string_table().offset(self.get().name_index() as isize) };

        // strlen without null byte
        let strlen = {
            let mut len = 0;
            while unsafe { *name_ptr.offset(len) } != 0 {
                len += 1;
            }
            len as usize
        };

        str::from_utf8(unsafe { slice::from_raw_parts(name_ptr, strlen) })
    }

    /// Get the physical start address of the section.
    pub fn start_address(&self) -> u64 {
        self.get().addr()
    }

    /// Get the physical end address of the section.
    ///
    /// This is the same as doing `section.start_address() + section.size()`
    pub fn end_address(&self) -> u64 {
        self.get().addr() + self.get().size()
    }

    /// Get the section's size in bytes.
    pub fn size(&self) -> u64 {
        self.get().size()
    }

    /// Get the section's address alignment constraints.
    ///
    /// That is, the value of `start_address` must be congruent to 0,
    /// modulo the value of `addrlign`. Currently, only 0 and positive
    /// integral powers of two are allowed. Values 0 and 1 mean the section has no
    /// alignment constraints.
    pub fn addralign(&self) -> u64 {
        self.get().addralign()
    }

    unsafe fn string_table(&self) -> *const u8 {
        let addr = match self.entry_size {
            40 => (*(self.string_section as *const ElfSectionInner32)).addr as usize,
            64 => (*(self.string_section as *const ElfSectionInner64)).addr as usize,
            s => panic!("Unexpected entry size: {}", s),
        };
        addr as *const _
    }
}

/// All types of ELF sections
#[derive(Iternum, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u32)]
pub enum ElfSectionType {
    /// This value marks the section header as inactive; it does not have an
    /// associated section. Other members of the section header have undefined
    /// values.
    Unused = 0,

    /// The section holds information defined by the program, whose format and
    /// meaning are determined solely by the program.
    ProgramSection = 1,

    /// This section holds a linker symbol table.
    LinkerSymbolTable = 2,

    /// The section holds a string table.
    StringTable = 3,

    /// The section holds relocation entries with explicit addends, such as type
    /// Elf32_Rela for the 32-bit class of object files. An object file may have
    /// multiple relocation sections.
    RelaRelocation = 4,

    /// The section holds a symbol hash table.
    SymbolHashTable = 5,

    /// The section holds dynamic linking tables.
    DynamicLinkingTable = 6,

    /// This section holds information that marks the file in some way.
    Note = 7,

    /// A section of this type occupies no space in the file but otherwise resembles
    /// `ProgramSection`. Although this section contains no bytes, the
    /// sh_offset member contains the conceptual file offset.
    Uninitialized = 8,

    /// The section holds relocation entries without explicit addends, such as type
    /// Elf32_Rel for the 32-bit class of object files. An object file may have
    /// multiple relocation sections.
    RelRelocation = 9,

    /// This section type is reserved but has unspecified semantics.
    Reserved = 10,

    /// This section holds a dynamic loader symbol table.
    DynamicLoaderSymbolTable = 11,

    /// Values in this inclusive range (`[0x6000_0000, 0x6FFF_FFFF)`) are
    /// reserved for environment-specific semantics.
    EnvironmentSpecific = 0x6000_0000,

    /// Values in this inclusive range (`[0x7000_0000, 0x7FFF_FFFF)`) are
    /// reserved for processor-specific semantics.
    ProcessorSpecific = 0x7000_0000,
}

/// An iterator over elf sections
#[derive(Clone)]
pub struct SectionIter {
    current_section: *const u8,
    remaining_sections: u32,
    entry_size: u32,
    string_section: *const u8,
}

impl Iterator for SectionIter {
    type Item = ElfSection;

    fn next(&mut self) -> Option<Self::Item> {
        while self.remaining_sections != 0 {
            let section = ElfSection {
                inner: self.current_section,
                string_section: self.string_section,
                entry_size: self.entry_size,
            };

            self.current_section = unsafe { self.current_section.offset(self.entry_size as isize) };
            self.remaining_sections -= 1;

            if section.section_type() != ElfSectionType::Unused {
                return Some(section)
            }
        }
        None
    }
}

impl Debug for SectionIter {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        let mut debug = f.debug_list();
        self.clone().for_each(|ref e| {
            debug.entry(e);
        });
        debug.finish()
    }
}

impl Default for SectionIter {
    fn default() -> Self {
        Self {
            current_section: core::ptr::null(),
            remaining_sections: 0,
            entry_size: 0,
            string_section: core::ptr::null(),
        }
    }
}

/// Inner parameters of elf section. All inside methods are getters.
pub trait ElfSectionInner {
    fn name_index(&self) -> u32;
    fn typ(&self) -> u32;
    fn flags(&self) -> u64;
    fn addr(&self) -> u64;
    fn size(&self) -> u64;
    fn addralign(&self) -> u64;
}

#[derive(Clone, Copy, Debug)]
#[repr(C, packed)]
pub struct ElfSectionInner32 {
    name_index: u32,
    typ: u32,
    flags: u32,
    addr: u32,
    offset: u32,
    size: u32,
    link: u32,
    info: u32,
    addralign: u32,
    entry_size: u32,
}

#[derive(Clone, Copy, Debug)]
#[repr(C, packed)]
pub struct ElfSectionInner64 {
    name_index: u32,
    typ: u32,
    flags: u64,
    addr: u64,
    offset: u64,
    size: u64,
    link: u32,
    info: u32,
    addralign: u64,
    entry_size: u64,
}

impl ElfSectionInner for ElfSectionInner32 {
    fn name_index(&self) -> u32 {
        self.name_index
    }

    fn typ(&self) -> u32 {
        self.typ
    }

    fn flags(&self) -> u64 {
        self.flags.into()
    }

    fn addr(&self) -> u64 {
        self.addr.into()
    }

    fn size(&self) -> u64 {
        self.size.into()
    }

    fn addralign(&self) -> u64 {
        self.addralign.into()
    }
}

impl ElfSectionInner for ElfSectionInner64 {
    fn name_index(&self) -> u32 {
        self.name_index
    }

    fn typ(&self) -> u32 {
        self.typ
    }

    fn flags(&self) -> u64 {
        self.flags
    }

    fn addr(&self) -> u64 {
        self.addr
    }

    fn size(&self) -> u64 {
        self.size
    }

    fn addralign(&self) -> u64 {
        self.addralign
    }
}
