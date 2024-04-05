use alloc::borrow::ToOwned;
use core::mem;

/// Custom module for communication with ACPI.
///
/// ACPI allows power management and provides different configurations for the OS
/// to handle, like the amount of running threads for example. ACPI contains of
/// different tables like RSDP, BGRT, FADT etc.

use crate::kernel_components::os::UChar;
pub use super::{
    rsdt::{RSDT, XSDT},
};

/// Custom trait for all system description tables defined by ACPI specification.
///
///
pub trait SystemDescriptionTable {
    /// Signature defined by 'signature' field in ACPISDTHeader structure. Each SDT has it's
    /// own signature and it is written in memory in table headers.
    const SIGNATURE: &'static str;

    /// Performs full SDT validation and will provide info about any error that occur.
    ///
    /// Validate SDT based on it's checksum provided in ACPI DST Header. Errors can be 
    /// returned based on table type. All tables must be validated, even if they were obtained 
    /// by the link of other tables.
    fn validate(header: &ACPISDTHeader) -> Result<(), SDTValidationError> {
        use SDTValidationError::*;

        if !Self::checksum(header) {
            return Err(CHECKSUM)
        }

        Ok(())
    }

    /// Makes checksum manually, Return true if everything is right.
    fn checksum(header: &ACPISDTHeader) -> bool {
        let slice: &[u8] = unsafe {
            core::slice::from_raw_parts(
                header as *const _ as *const u8,
                mem::size_of::<ACPISDTHeader>(),
            )
        };

        // All bytes in the structure must sum up to zero.
        slice.iter().fold(0u8, |s, b| s.wrapping_add(*b)) == 0
    }
}

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
    pub signature: Signature,
    /// The size of the entire table.
    length: u32,
    /// The revision of the ACPI.
    revision: u8,
    /// All bytes of the table summed must be equal to 0. 
    checksum: u8,
    pub oem_id: [UChar; 6],
    pub oem_table_id: [UChar; 8],
    pub oem_revision: u32,
    creator_id: u32,
    creator_revision: u32,
}

/// All ACPI tables have a 4 byte Signature field, except the RSDP which has
/// an 8 byte one. This signature is used when the OS must determine which table
/// it is useing at this moment.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Signature {
    /// Most ACPI tables use this.
    Default([UChar; 4]),
    /// Special only for RSDT table.
    RSDT([UChar; 8]),
} 

impl PartialEq<str> for Signature {
    fn eq(&self, other: &str) -> bool {
        match self {
            Self::Default(b) => b == other.as_bytes(),
            Self::RSDT(b) => b == other.as_bytes(),
        }
    }
}

/// Custom defined errors, which may occur when validating tables.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SDTValidationError {
    /// 8 bit checksum field is wrong. 
    /// All bytes of the table summed must be equal to 0
    CHECKSUM,
}
