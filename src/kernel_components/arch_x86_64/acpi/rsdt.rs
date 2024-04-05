use alloc::borrow::ToOwned;
/// Custom module that implements RSDT table. It contains pointers to all system
/// description tables.

use alloc::vec::Vec;
use super::acpi::{ACPISDTHeader, SDTValidationError, Signature, SystemDescriptionTable};
use super::rsdp::{RSDP, XSDP};
use crate::critical_section;

use core::ptr;

pub use super::rsdp::{ACPITagNew, ACPITagOld};

/// Root/Extended Root System Description Table.
///
/// Data structure used in the ACPI programming interface, which contains pointers
/// to all other SDTs. This is a legacy version of newer XSDT table, which use 32-bit
/// addresses so it is not used anymore.
#[repr(C)]
#[derive(Debug, Clone)]
pub struct RSDT {
    /// RSDT has 8-byte signature header.
    header: ACPISDTHeader,
    /// Pointers for ACPI version 1.0
    ptrs: [*mut u32; 1],
}

impl SystemDescriptionTable for RSDT {
    const SIGNATURE: &'static str = "RSDT";
}

impl RSDT {
    /// Reads the current value of RSDT.
    ///
    /// If ACPI < 2.0 is used, this version is required.
    pub fn new() -> Self {
        // Critical section, because reading from the MMU.
        let rsdp = critical_section!(|| {
            RSDP::new()
        }).expect("Unable to read from RSDP.");

        unsafe { ptr::read_unaligned(rsdp.ptr as *mut RSDT) } 
    }
    
    /// Finds the requested SDT table. 
    ///
    /// This function does all validation checks and return the result accordingly.
    ///
    /// # Returns
    ///
    /// Will return an error if table was found but its validation failed. Will return
    /// Ok(None) if table was not found for some reason. Will return Ok(&T), where T is
    /// expected to be another SDT. 
    fn find<T>(&self) -> Result<Option<&mut T>, SDTValidationError> where 
        T: SystemDescriptionTable
    {
        for ptr in self.ptrs.into_iter().map(|ptr| ptr.cast::<ACPISDTHeader>()) {
            if let Some(header) = unsafe { ptr.as_ref() } {
                // If signature matches the header signature, checking the obtained SDT and
                // returning it if validated.
                if header.signature == *T::SIGNATURE {
                    // No it is necessary to validate the header fully.
                    match T::validate(header) {
                        Ok(_) => {
                            let sdt = unsafe {
                                // Here we are free to cast the header pointer as the SDT.
                                ptr.cast::<T>().as_mut().unwrap()
                            };
                            return Ok(Some(sdt));
                        },
                        Err(e) => return Err(e),
                    }
                }
            }
        }

        Ok(None)
    }
}


/// Extended Root System Description Table.
///
/// Data structure used in the ACPI programming interface 2.0, which contains pointers
/// to all other SDTs. This version is used in x86_64 systems as a substitution of legacy
/// 1.0 RSDT.
#[derive(Debug, Clone)]
#[repr(C)]
pub struct XSDT {
    /// XSDT has 8-byte signature header.
    header: ACPISDTHeader,
    /// Pointers for ACPI version 2.0
    ptrs: [*mut usize; 1],
}

impl SystemDescriptionTable for XSDT {
    const SIGNATURE: &'static str = "XSDT";
}

impl XSDT {
    /// Reads the current value of XSDT.
    ///
    /// If ACPI 2.0 is used, this version is required.
    pub fn new() -> Self {
        // Critical section, because reading from the MMU.
        let xsdp = critical_section!(|| {
            XSDP::new()
        }).expect("Unable to read from XSDP.");

        unsafe { ptr::read_unaligned(xsdp.ptr as *mut XSDT) }
    } 

    /// Finds the requested SDT table. 
    ///
    /// This function does all validation checks and return the result accordingly.
    ///
    /// # Returns
    ///
    /// Will return an error if table was found but its validation failed. Will return
    /// Ok(None) if table was not found for some reason. Will return Ok(&T), where T is
    /// expected to be another SDT. 
    fn find<T>(&self) -> Result<Option<&mut T>, SDTValidationError> where 
        T: SystemDescriptionTable
    {
        for ptr in self.ptrs.into_iter().map(|ptr| ptr.cast::<ACPISDTHeader>()) {
            if let Some(header) = unsafe { ptr.as_ref() } {
                // If signature matches the header signature, checking the obtained SDT and
                // returning it if validated.
                if header.signature == *T::SIGNATURE {
                    // No it is necessary to validate the header fully.
                    match T::validate(header) {
                        Ok(_) => {
                            let sdt = unsafe {
                                // Here we are free to cast the header pointer as the SDT.
                                ptr.cast::<T>().as_mut().unwrap()
                            };
                            return Ok(Some(sdt));
                        },
                        Err(e) => return Err(e),
                    }
                }
            }
        }

        Ok(None)
    }

}
