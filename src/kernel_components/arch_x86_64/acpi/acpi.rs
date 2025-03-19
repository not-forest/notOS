
/// Custom module for communication with ACPI.
///
/// ACPI allows power management and provides different configurations for the OS
/// to handle, like the amount of running threads for example. ACPI contains of
/// different tables like RSDP, BGRT, FADT etc.

use core::{error::Error, fmt::Display};
use alloc::{borrow::ToOwned, fmt, string::String};
use crate::{
    bitflags, 
    kernel_components::os::UChar
};

/* Useful descriptor table reexport. */
pub use super::{
    rsdt::{RSDT, XSDT},
    fadt::{FADT, PPMP},
    madt::MADT,
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

        // Signature check.
        if header.signature != *Self::SIGNATURE.as_bytes() {
            return Err(Signature(header.signature))
        }
        // Checksum check.
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
                header.length as usize,
            )
        };

        // All bytes in the structure must sum up to zero.
        slice.iter().fold(0u8, |s, b| s.wrapping_add(*b)) == 0
    }

    /// Additional calibration function.
    ///
    /// Used for tables, which dont have a well defined constant length. This function is
    /// automatically called when obtaining the table via XSDT/RSDT. For most tables this function
    /// does nothing.
    fn calibrate(&mut self) {}
}

/// Advanced Configuration and Power Interface.
///
/// This struct virtualize all interactions with ACPI interface and it's tables by
/// using special dedicated functions. ACPI consists of two main parts:
/// - Tables used by the OS for configuration during the boot;
/// - Run time ACPI environment to interract with system management code;
pub mod acpi_service {
    use crate::kernel_components::arch_x86_64::{
        ports::{GenericPort, PortAccessType},
        acpi::FADT,
    };

    #[macro_use]
    macro_rules! acpi_error {
        () => ($crate::error!());
        ($fmt:expr) => ($crate::error!(concat!("ACPI ERROR: " ,$fmt)));
        ($fmt:expr, $($arg:tt)*) => ($crate::error!(concat!("ACPI ERROR: ", $fmt), $($arg)*));
    }

    #[cfg(feature = "virt_qemu")] const QEMU_PORT_OLD: u16 = 0xb004;
    #[cfg(feature = "virt_qemu")] const QEMU_PORT_NEW: u16 = 0x604;

    /// Custom result type for ACPI service. Different functions within the service module may
    /// return some custom type or ACPIError.
    type ACPIResult<T> = Result<T, ACPIError>;

    /// Shutdowns the machine in a proper way.
    ///
    /// # Warn
    ///
    /// This function have no effect on any running processes, therefore a higher order
    /// process must be used to call a cleanup procedure from the PMU and then calling this
    /// function.
    ///
    /// # Virtualization
    ///
    /// This function won't call a regular shutdown procedure, when the OS is virtualized via
    /// some virtual machine like QEMU.
    pub fn shutdown(fadt: Option<&FADT>) -> ACPIResult<()> {
        // This part will only be compiled for qemu.
        #[cfg(feature = "virt_qemu")] {
            GenericPort::new(QEMU_PORT_OLD, PortAccessType::WRITEONLY)
                .write(0x2000 as u16); // Old QEMU. 
            GenericPort::new(QEMU_PORT_NEW, PortAccessType::WRITEONLY)
                .write(0x2000 as u16); // Newer QEMU. 
        }

        // This code will be executed on a regular target.
        #[cfg(not(feature = "virt_qemu"))] {
            // Extracting the FADT.
            let fadt = fadt.expect("FADT argument cannot be 'NONE' for non virtualized build.");
            // Obtaining DSDT and interpreting AML code.
            match fadt.dsdt() {
                Ok(dsdt) => crate::println!("{:#?}", dsdt.aml().0.len()),
                Err(err) => {
                    acpi_error!("FATAL ERROR: Unable to obtain DSDT table: {}", err);
                    return Err(ACPIError::DSDT); 
                }
            }
            unimplemented!() //TODO! implement DSDT parsing and interpreting shutdown code.
        }
    }

    /// Custom error type for ACPI service.
    ///
    /// Those error codes contain info about what went wrong when calling some ACPI service within
    /// this module. Usually most errors related to not being able to obtain one or more ACPI
    /// tables. This is why error code just defines which table it was unable to obtain and throws
    /// it back.
    pub enum ACPIError {
        RSDT, XSDT, FADT, DSDT, SSDT
    }
}

/// ACPI SDT Table header.
///
/// First part of every table structures related to ACPI. All ACPI SDTs may be splitted
/// in two parts. This is the first part which is the same for all of them, with different
/// minor changes.
#[repr(C, align(0x4))]
#[derive(Debug, Clone, Copy)]
pub struct ACPISDTHeader {
    /// 4 byte signature field, which defines which table is being used.
    pub signature: [UChar; 4],
    /// The size of the entire table in bytes.
    pub length: u32,
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

/// Additional structure that represent an extended address to the position of 
/// different registers in ACPI. It provides the platform with a robust means 
/// to describe register locations. This structure is used to express register 
/// addresses within tables defined by ACPI.
#[derive(Debug, Clone, Copy)]
pub(crate) struct GenericAddressStructure {
    /// The address space where the data structure or register exists.
    addr_space: u8,
    /// The size in bits of the given register. When addressing a data structure,
    /// this field must stay zero.
    bit_width: u8,
    /// The bit offset of the given register at the given address. When addressing 
    /// a data structure, this field must stay zero.
    bit_offset: u8,
    /// Specifies access size. 
    access_size: u8,
    /// 64-bit address of the data structure or register in the fiven address space.
    addr: usize,
}

/// Custom defined errors, which may occur when validating tables.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SDTValidationError {
    /// 8 bit checksum field is wrong. All bytes of the table summed must be equal to 0
    CHECKSUM,
    /// The signature does not match the trait's signature and therefore wrong.
    Signature([UChar; 4]),
}

impl Error for SDTValidationError {}

impl Display for SDTValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CHECKSUM => write!(f, "Table's checksum doesn't match."),
            Self::Signature(s) => write!(f, "Table's signature doesn't match: {:?}", s),
        }
    }
}

/// Specifies access sizze. Unless otherwise defined by the Address Space ID.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessSize {
    UNDEFINED, BYTE, WORD, DWORD, QWORD 
}

bitflags! {
    /// The Generic Address Structure (GAS) 
    ///
    /// It provides the platform with a robust means to describe register locations. This structure 
    /// is used to express register addresses within tables defined by ACPI .
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct AddressSpaceID: u8 {
        /// The 64-bit physical memory address (relative to the processor) of the register. 32-bit 
        /// platforms must have the high DWORD set to 0.
        const SYSTEM_MEMORY_SPACE               = 0x00,
        /// The 64-bit I/O address (relative to the processor) of the register. 32-bit platforms 
        /// must have the high DWORD set to 0.
        const SYSTEM_IO_SPACE                   = 0x01,
        /// PCI Configuration space addresses must be confined to devices on PCI Segment Group 0, 
        /// bus 0. This restriction exists to accommodate access to fixed hardware prior to PCI 
        /// bus enumeration. The format of addresses are defined as follows:
        /// - highest reserved word (must be 0)
        /// - PCI device number on bus 0
        /// - PCI function number
        ///
        /// Longest Word Offset in the configuration space header For example: Offset 23h of Function 
        /// 2 on device 7 on bus 0 segment 0 would be represented as: 0x0000000700020023.
        const PCI_CONFIGUIRATION_SPACE          = 0x02,
        const EMBEDDED_CONTROLLER               = 0x03,
        const SMBUS                             = 0x04,
        const SYSTEM_CMOS                       = 0x05,
        /// PciBarTarget is used to locate a MMIO register on a PCI device BAR space.
        const PCI_BAR_TARGET                    = 0x06,
        const IPMI                              = 0x07,
        const GENERAL_PURPOSE_IO                = 0x08,
        const GENERIC_SERIAL_BUS                = 0x09,
        const PLATFORM_COMMUNICATION_CHANNEL    = 0x0a,
        // Addresses 0x0b to 0x7e are reserved.
        const FUNCTIONAL_FIXED_HARDWARE         = 0x7f,
        // Addresses 0x80 t0 0xbf are reserved.
    };
}
