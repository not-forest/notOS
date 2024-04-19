//! Module that defines root system description pointers as both structure and tag.

use super::acpi::{ACPISDTHeader, Signature, SystemDescriptionTable};
use crate::kernel_components::memory::memory_module::MEMORY_MANAGEMENT_UNIT;
use crate::kernel_components::memory::tags::{Tag, TagTrait, TagType, TagTypeId};
use crate::kernel_components::os::UChar;

use core::mem;

/// Defines a set of errors that can occur while validating the RSDP or XSDP.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootPointerError {
    /// Pointer had an invalid checksum.
    CHECKSUM,
    /// The signature is wrong.
    SIGNATURE,
    /// Unable to retrieve the Tag.
    NOTAG,
}

/// Special pointer which points to the RSDT structure in ACPI version 1.0
#[repr(C, packed)]
#[derive(Debug, Clone)]
pub(crate) struct RSDP {
    signature: [UChar; 8],
    /// Version 1.0 checksum
    checksum: u8,
    /// OEM-supplied string that identifies the OEM
    oem_id: [UChar; 6],
    /// SDTs revision.
    revision: u8,
    /// An actual pointer to the RSDT structure. Deprecated since version 2.0
    pub ptr: u32,
}

impl RSDP {
    /// Obtains the RSDP pointer from the MMU.
    pub(crate) fn new() -> Result<Self, RootPointerError> {
        if let Some(tag) = unsafe { MEMORY_MANAGEMENT_UNIT.get_rsdp() } { 
            let rsdp = tag.rsdp.clone();

            // Validating the rsdp right away.
            match rsdp.validate() {
                Ok(_) => Ok(rsdp),
                Err(e) => Err(e),
            }
        } else {
            Err(RootPointerError::NOTAG)
        }
    }

    /// Returns true if checksum is valid.
    pub fn checksum(&self) -> bool {
        let slice: &[u8] = unsafe {
            core::slice::from_raw_parts(
                self as *const _ as *const u8,
                mem::size_of::<Self>(),
            )
        };

        // All bytes in the structure must sum up to zero.
        slice.iter().fold(0u8, |s, b| s.wrapping_add(*b)) == 0
    }

    /// Validates the RSDP pointer.
    pub fn validate(&self) -> Result<(), RootPointerError> {
        // Signature check.
        if self.signature != *b"RSD PTR " {
            return Err(RootPointerError::SIGNATURE)
        }

        // Checksum check.
        if !self.checksum() {
            return Err(RootPointerError::CHECKSUM)
        }

        Ok(())
    }
}

/// Special pointer which points to the XSDT structure in ACPI version 2.0
#[repr(C, packed)]
#[derive(Debug, Clone)]
pub(crate) struct XSDP {
    /// All fields in RSDP are also present in XSDT, hovewer the ptr field is deprecated.
    signature: [UChar; 8],
    /// Version 1.0 checksum
    _checksum_legacy: u8,
    /// OEM-supplied string that identifies the OEM
    oem_id: [UChar; 6],
    /// SDTs revision.
    revision: u8,
    /// Deprecated since version 2.0
    _ptr_legacy: u32,

    /// The size of the entire table since offset 0 to the end.
    length: u32,
    /// An actual pointer to the XSDT structure.
    pub ptr: u64,
    /// Extended checksum.
    checksum: u8,
    _reserved: [u8; 3],
}

impl XSDP {
    /// Obtains the XSDP pointer from the MMU.
    pub(crate) fn new() -> Result<Self, RootPointerError> {
        if let Some(tag) = unsafe { MEMORY_MANAGEMENT_UNIT.get_xsdp() } { 
            let xsdp = tag.xsdp.clone();

            // Validating the rsdp right away.
            match xsdp.validate() {
                Ok(_) => Ok(xsdp),
                Err(e) => Err(e),
            }
        } else {
            Err(RootPointerError::NOTAG)
        }
    }

    /// Returns true if checksum is valid.
    pub fn checksum(&self) -> bool {
        let slice: &[u8] = unsafe {
            core::slice::from_raw_parts(
                self as *const _ as *const u8,
                self.length as usize,
            )
        };

        // All bytes in the structure must sum up to zero.
        slice.iter().sum::<u8>() == 0
    }

    /// Validates the XSDP pointer.
    pub fn validate(&self) -> Result<(), RootPointerError> {
        // Signature check.
        if self.signature != *b"RSD PTR " {
            return Err(RootPointerError::SIGNATURE)
        }

        // Checksum check.
        if !self.checksum() {
            return Err(RootPointerError::CHECKSUM)
        }

        Ok(())
    }
}

/// Tag which contains a copy of RSDP pointer for ACPI v1.0
///
/// This is the tag from multiboot2 structure. It must be used to obtain the RSDP
/// pointer for legacy ACPI v1.0
#[derive(Clone)]
#[repr(C)]
pub struct ACPITagOld {
    tag_type: TagTypeId,
    size: u32,
    pub rsdp: RSDP,
}

impl TagTrait for ACPITagOld {
    const ID: TagType = TagType::AcpiOld;
    fn dst_size(tag: &Tag) {}
}

/// Tag which contains a copy of XSDP pointer for ACPI v2.0
///
/// This is the tag from multiboot2 structure. It must be used to obtain the XSDP
/// pointer for legacy ACPI v2.0
#[derive(Clone)]
#[repr(C)]
pub struct ACPITagNew {
    tag_type: TagTypeId,
    size: u32,
    pub xsdp: XSDP,
}

impl TagTrait for ACPITagNew {
    const ID: TagType = TagType::AcpiNew;
    fn dst_size(tag: &Tag) {}
}
