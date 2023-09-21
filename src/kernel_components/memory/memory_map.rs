/// A memory map tag and it's corresponding structures and methods.
use core::fmt::{Debug, Formatter};
use core::marker::PhantomData;
use core::mem;
use core::ops::Deref;

use proc_macros::Iternum;
use crate::{Vec, AsBytes};
use crate::kernel_components::structures::boxed::BoxedDst;

use super::tags::{TagTrait, TagType, TagTypeId, Tag};

const METADATA_SIZE: usize = mem::size_of::<TagTypeId>() + 3 * mem::size_of::<u32>();

impl AsBytes for u32 {}

/// This tag provides an initial host memory map (legacy boot, not UEFI).
///
/// The map provided is guaranteed to list all standard RAM that should be
/// available for normal use. This type however includes the regions occupied
/// by kernel, mbi, segments and modules. Kernel must take care not to
/// overwrite these regions.
///
/// This tag may not be provided by some boot loaders on EFI platforms if EFI
/// boot services are enabled and available for the loaded image (The EFI boot
/// services tag may exist in the Multiboot2 boot information structure).
#[derive(Debug, PartialEq, Eq)]
#[repr(C)]
pub struct MemoryMapTag {
    typeid: TagTypeId,
    size: u32,
    entry_size: u32,
    entry_version: u32,
    areas: [MemoryArea],
}

impl MemoryMapTag {
    /// Creates a new memory map tag.
    pub fn new(areas: &[MemoryArea]) -> BoxedDst<Self> {
        let entry_size: u32 = mem::size_of::<MemoryArea>().try_into().unwrap();
        let entry_version: u32 = 0;
        let mut bytes = Vec::from_array(&[entry_size.as_bytes(), entry_version.as_bytes()]);

        for area in areas {
            bytes.push(area.as_bytes());
        }

        BoxedDst::new_tag_dst(bytes.as_bytes().into())
    }

    /// Returns the entry size.
    pub fn entry_size(&self) -> u32 {
        self.entry_size
    }

    /// Returns the entry version.
    pub fn entry_version(&self) -> u32 {
        self.entry_version
    }

    /// Return the slice with all memory areas.
    pub fn memory_areas(&self) -> &[MemoryArea] {
        &self.areas
    }

    /// Converts areas in the memory map tag into 'MemoryAreaIter'
    pub fn memory_map_iter(&self) -> MemoryAreaIter {
        let self_ptr = self as *const MemoryMapTag;
        let start_area = (&self.areas[0]) as *const MemoryArea;
        MemoryAreaIter {
            current_area: start_area as u64,
            last_area: (self_ptr as *const () as u64 + self.size as u64 - 1),
            entry_size: self.entry_size,
            phantom: PhantomData,
        }
    }
}

impl TagTrait for MemoryMapTag {
    const ID: TagType = TagType::Mmap;

    fn dst_size(base_tag: &Tag) -> usize {
        assert!(base_tag.size as usize >= METADATA_SIZE);
        let size = base_tag.size as usize - METADATA_SIZE;
        assert_eq!(size % mem::size_of::<MemoryArea>(), 0);
        size / mem::size_of::<MemoryArea>()
    }
}

/// A memory area entry descriptor.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(C)]
pub struct MemoryArea {
    pub base_addr: u64,
    pub length: u64,
    typ: MemoryAreaTypeId,
    _reserved: u32,
}

/// Implements byte representation of MemoryArea struct.
impl AsBytes for MemoryArea {}

impl MemoryArea {
    /// Create a new MemoryArea.
    pub fn new(base_addr: u64, length: u64, typ: impl Into<MemoryAreaTypeId>) -> Self {
        Self {
            base_addr,
            length,
            typ: typ.into(),
            _reserved: 0,
        }
    }

    /// The start address of the memory region.
    pub fn start_address(&self) -> u64 {
        self.base_addr
    }

    /// The end address of the memory region.
    pub fn end_address(&self) -> u64 {
        self.base_addr + self.length
    }

    /// The size, in bytes, of the memory region.
    pub fn size(&self) -> u64 {
        self.length
    }

    /// The type of the memory region.
    pub fn typ(&self) -> MemoryAreaTypeId {
        self.typ
    }
}

impl Debug for MemoryArea {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("MemoryArea")
            .field("base_addr", &self.base_addr)
            .field("length", &self.length)
            .field("typ", &self.typ)
            .finish()
    }
}

// ABI-friendly version of [`MemoryAreaType`].
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(C)]
pub struct MemoryAreaTypeId(u32);

impl From<u32> for MemoryAreaTypeId {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

impl From<MemoryAreaTypeId> for u32 {
    fn from(value: MemoryAreaTypeId) -> Self {
        value.0
    }
}

impl Debug for MemoryAreaTypeId {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        let mt = MemoryAreaType::from(*self);
        Debug::fmt(&mt, f)
    }
}

/// Abstraction over defined memory types for the memory map as well as custom
/// ones. Types 1 to 5 are defined in the Multiboot2 spec and correspond to the
/// entry types of e820 memory maps.
#[derive(Iternum, Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MemoryAreaType {
    /// Available memory free to be used by the OS.
    Available,
    /// A reserved area that must not be used.
    Reserved,
    /// Usable memory holding ACPI information.
    AcpiAvailable,
    /// Reserved memory which needs to be preserved on hibernation.
    /// Also called NVS in spec, which stands for "Non-Volatile Sleep/Storage",
    /// which is part of ACPI specification.
    ReservedHibernate,
    /// Memory which is occupied by defective RAM modules.
    Defective,
    /// Custom memory map type.
    Custom(u32),
}

#[derive(Clone, Debug)]
pub struct MemoryAreaIter {
    current_area: u64,
    last_area: u64,
    entry_size: u32,
    phantom: PhantomData<&'static MemoryArea>
}

impl Iterator for MemoryAreaIter {
    type Item = &'static MemoryArea;
    fn next(&mut self) -> Option<Self::Item> {
        if self.current_area > self.last_area {
            None
        } else {
            let area = unsafe { &*(self.current_area as *const MemoryArea) };
            self.current_area += self.entry_size as u64;
            Some(area)
        }
    }
}

impl From<MemoryAreaTypeId> for MemoryAreaType {
    fn from(value: MemoryAreaTypeId) -> Self {
        match value.0 {
            0..=5 => Self::get_variant(value.0 as usize),
            val => Self::Custom(val),
        }
    }
}

impl From<MemoryAreaType> for MemoryAreaTypeId {
    fn from(value: MemoryAreaType) -> Self {
        let integer = match value {
            MemoryAreaType::Custom(val) => val,
            _ => MemoryAreaType::get_index(value) as u32,
        };
        integer.into()
    }
}

impl PartialEq<MemoryAreaType> for MemoryAreaTypeId {
    fn eq(&self, other: &MemoryAreaType) -> bool {
        let val: MemoryAreaTypeId = (*other).into();
        let val: u32 = val.0;
        self.0.eq(&val)
    }
}

impl PartialEq<MemoryAreaTypeId> for MemoryAreaType {
    fn eq(&self, other: &MemoryAreaTypeId) -> bool {
        let val: MemoryAreaTypeId = (*self).into();
        let val: u32 = val.0;
        other.0.eq(&val)
    }
}
