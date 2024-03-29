/// Custom module that implements RSDT table. It contains pointers to all system
/// description tables.

use alloc::vec::Vec;
use super::acpi::{ACPISDTHeader, Signature, OEMId};
use crate::kernel_components::memory::{
    tags::{TagTrait, TagType, TagTypeId, Tag},
    memory_module::MEMORY_MANAGEMENT_UNIT,
};
use crate::critical_section;

use core::ptr;

/// Root/Extended Root System Description Table.
///
/// Data structure used in the ACPI programming interface, which contains pointers
/// to all other SDTs. It might have different size of it's fields based on the
/// current ACPI version.
#[derive(Debug, Clone)]
pub struct RXSDT {
    /// RSDT has 8-byte signature header.
    header: ACPISDTHeader,
    /// Pointers to other system description tables.
    ptrs: SDTPointers,
}

impl RXSDT {
    /// Reads the current value of RSDT.
    ///
    /// If ACPI < 2.0 is used, this version is required.
    pub unsafe fn new_rsdt() -> Self {
        // Critical section, because reading from the MMU.
        let rsdp = critical_section!(|| {
            RSDP::new()
        }).expect("Unable to read from RSDP.");

        ptr::read(rsdp.ptr as *mut RXSDT) 
    }

    /// Reads the current value of XSDT.
    ///
    /// If ACPI 2.0 is used, this version is required.
    pub unsafe fn new_xsdt() -> Self {
        // Critical section, because reading from the MMU.
        let xsdp = critical_section!(|| {
            XSDP::new()
        }).expect("Unable to read from XSDP.");

        ptr::read(xsdp.ptr as *mut RXSDT)
    }
}

/// Special pointer which points to the RSDT structure in ACPI version 1.0
#[repr(C)]
#[derive(Debug, Clone)]
struct RSDP {
    signature: [char; 8],
    /// Version 1.0 checksum
    checksum: u8,
    /// OEM-supplied string that identifies the OEM
    oem_id: OEMId,
    /// SDTs revision.
    revision: u8,
    /// An actual pointer to the RSDT structure. Deprecated since version 2.0
    ptr: u32,
}

impl RSDP {
    /// Obtains the RSDP pointer from the MMU.
    fn new() -> Option<Self> {
        if let Some(tag) = unsafe { MEMORY_MANAGEMENT_UNIT.get_rsdp() } { 
            crate::println!("{:#?}, {:#?}, {:#?}", tag.size, tag.tag_type, tag.rsdp);
            return Some(tag.rsdp.clone())
        }
        None
    }
}

/// Special pointer which points to the XSDT structure in ACPI version 2.0
#[repr(C)]
#[derive(Debug, Clone)]
struct XSDP {
    /// All fields in RSDP are also present in XSDT, hovewer the ptr field is deprecated.
    legacy: RSDP,
    /// The size of the entire table since offset 0 to the end.
    length: u32,
    /// An actual pointer to the XSDT structure.
    ptr: u64,
    /// Extended checksum.
    xchecksum: u8,
    _reserved: [u8; 3],
}

impl XSDP {
    /// Obtains the RSDP pointer from the MMU.
    fn new() -> Option<Self> {
        if let Some(tag) = unsafe { MEMORY_MANAGEMENT_UNIT.get_xsdp() } {
            crate::println!("{:#?}, {:#?}, {:#?}", tag.size, tag.tag_type, tag.xsdp);
            return Some(tag.xsdp.clone())
        }
        None
    }
}

/// Enum that consists of pointers to other SDTs.
///
/// The amount of those pointers vary, because RSDT can be obtained either with
/// RSDP or XSDP. This is different in different versions of ACPI.
#[derive(Debug, Clone)]

enum SDTPointers {
    /// Pointers for ACPI version 1.0
    Ver1(Vec<u32>),
    /// Pointers for ACPI version 2.0
    Ver2(Vec<usize>),
}

/// Tag which contains a copy of RSDP pointer for ACPI v1.0
///
/// This is the tag from multiboot2 structure. It must be used to obtain the RSDP
/// pointer for legacy ACPI v1.0
#[derive(Clone)]
#[repr(C)]
pub struct ACPITagOld {
    pub tag_type: TagTypeId,
    pub size: u32,
    rsdp: RSDP,
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
    pub tag_type: TagTypeId,
    pub size: u32,
    xsdp: XSDP,
}

impl TagTrait for ACPITagNew {
    const ID: TagType = TagType::AcpiNew;
    fn dst_size(tag: &Tag) {}
}
