/// Differentiated System Description Table
///
/// This module defines the DSDT table, which is used to descrive what peripherals the machine has,
/// power management and holds information on PCI IRQ mappings. 

use super::AMLStream;
use core::{mem, slice};
use crate::kernel_components::arch_x86_64::acpi::acpi::{
    SystemDescriptionTable, ACPISDTHeader
};

/// Differentiated System Description Table
///
/// As any ACPI table it consists of header and AML encoded code that describes machine's
/// pheripherals, PCI IRQ mappings and power management procedures.
#[repr(C)]
#[derive(Debug, Clone)]
pub struct DSDT {
    header: ACPISDTHeader
}

impl DSDT {
    /// Obtains the AML code slice from the DSDT table.
    ///
    /// Since DSDT is a table that consists of header and n-bytes of AML code. Since n is vendor
    /// specific, this method returns a slice of AML bytecode it form of AML stream structure.
    pub fn aml(&self) -> AMLStream {
        let ptr = self as *const _ as *const u8;
        unsafe {
            let aml_start = ptr.add(mem::size_of::<ACPISDTHeader>());
            let aml_end = ptr.add(self.header.length as usize);
            AMLStream(slice::from_ptr_range(aml_start..aml_end))
        }
    }
}

impl SystemDescriptionTable for DSDT {
    const SIGNATURE: &'static str = "DSDT";
}
