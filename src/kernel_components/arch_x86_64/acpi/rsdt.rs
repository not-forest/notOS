/// Custom module that implements RSDT table. It contains pointers to all system
/// description tables.

use alloc::borrow::ToOwned;
use alloc::boxed::Box;
use super::acpi::{ACPISDTHeader, SDTValidationError, Signature, SystemDescriptionTable};
use super::rsdp::{RSDP, XSDP};
use crate::critical_section;

use core::{mem, ptr};
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
    ptrs: &'static [*mut u32],
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

        RSDT::_ptrs_map(rsdp)
    }

    /// Works the same as new, but can be used to fast things up when you already do aquire
    /// a proper RSDP.
    pub(crate) unsafe fn from_rsdp(rsdp: RSDP) -> Self {
        RSDT::_ptrs_map(rsdp)
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
    pub fn find<T>(&self) -> Result<Option<&mut T>, SDTValidationError> where 
        T: SystemDescriptionTable
    {
        for ptr in self.ptrs.iter().map(|ptr| ptr.cast::<ACPISDTHeader>()) {
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

    /// Gets the amount of vectors this table has.
    pub fn ptrs_amount(&self) -> usize {
        _ptrs_amount(self.header)
    }

    fn _ptrs_map(rsdp: RSDP) -> Self {
        unsafe { 
            // Getting the amount of pointers for RSDT.
            let header = ptr::read_unaligned(rsdp.ptr as *mut ACPISDTHeader); 
            let ptrptr = rsdp.ptr as usize + mem::size_of::<ACPISDTHeader>();
            let amount = _ptrs_amount(header);

            RSDT {
                header, 
                ptrs: ptr::slice_from_raw_parts(
                    ptrptr as *const *mut u32, amount
                ).as_ref().unwrap()
            }
        } 
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
    ptrs: &'static [*mut usize],
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

        XSDT::_ptrs_map(xsdp)
    }

    /// Works the same as new, but can be used to fast things up when you already do aquire
    /// a proper RSDP.
    pub(crate) unsafe fn from_rsdp(xsdp: XSDP) -> Self {
        XSDT::_ptrs_map(xsdp)
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
    pub fn find<T>(&self) -> Result<Option<&mut T>, SDTValidationError> where 
        T: SystemDescriptionTable
    {
        for ptr in self.ptrs.iter().map(|ptr| ptr.cast::<ACPISDTHeader>()) {
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

    /// Gets the amount of vectors this table has.
    pub fn ptrs_amount(&self) -> usize {
        _ptrs_amount(self.header)
    }

    fn _ptrs_map(xsdp: XSDP) -> Self {
        unsafe { 
            // Getting the amount of pointers for RSDT.
            let header = ptr::read_unaligned(xsdp.ptr as *mut ACPISDTHeader); 
            let ptrptr = xsdp.ptr as usize + mem::size_of::<ACPISDTHeader>();
            let amount = _ptrs_amount(header);

            XSDT {
                header,
                ptrs: ptr::slice_from_raw_parts(
                    ptrptr as *const *mut usize, amount
                ).as_ref().unwrap()
            }
        } 
    }
}

fn _ptrs_amount(h: ACPISDTHeader) -> usize { 
    (h.length as usize - mem::size_of::<ACPISDTHeader>()) / 4 
}
