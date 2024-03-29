/// Custom module for communication with ACPI.
///
/// ACPI allows power management and provides different configurations for the OS
/// to handle, like the amount of running threads for example. ACPI contains of
/// different tables like RSDP, BGRT, FADT etc.

pub use super::{
    rsdt::RXSDT,
};

/// Advanced Configuration and Power Interface.
///
/// This struct virtualize all interactions with ACPI interface and it's tables by
/// using special dedicated functions. ACPI consists of two main parts:
/// - Tables used by the OS for configuration during the boot;
/// - Run time ACPI environment to interract with system management code;
pub struct ACPI;

/// ACPI SDT Table header.
///
/// First part of every table structures related to ACPI. All ACPI SDTs may be splitted
/// in two parts. This is the first part which is the same for all of them, with different
/// minor changes.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ACPISDTHeader {
    /// 4 byte or 8 byte signature field, which defines which table is being used.
    signature: Signature,
    /// The size of the entire table.
    length: u32,
    /// The revision of the ACPI.
    revision: u8,
    /// All bytes of the table summed butst be equal to 0. 
    checksum: u8,
    oem_id: OEMId,
    oem_table_id: OEMTableId,
    oem_revision: u32,
    creator_id: u32,
    creator_revision: u32,
}

/// All ACPI tables have a 4 byte Signature field, except the RSDP which has
/// an 8 byte one. This signature is used when the OS must determine which table
/// it is useing at this moment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Signature {
    /// Most ACPI tables use this.
    Default([char; 4]),
    /// Special only for RSDP table.
    RSDP([char; 8]),
} 

/// OEM-supplied string that identifies the OEM.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct OEMId([char; 6]);

/// OEM-supplied string that identifies the OEM table ID.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct OEMTableId([char; 8]);
