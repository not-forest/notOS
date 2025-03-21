
/// MADT table allows to query local APICs on every CPU core and manage interrupts.

use crate::{bitflags, kernel_components::os::OSChar};
use super::acpi::{ACPISDTHeader, SystemDescriptionTable};
use proc_macros::Iternum;
use core::{ptr, mem};

/// Multiple APIC Description Table
///
/// This ACPI table provides informations necessary for managing APIC, SAPIC, GIC or LPIC
/// controllers. It describes all of the interrupt controllers in the system and can be used to
/// enumerate currently available CPUs.
#[repr(C, packed)]
#[derive(Debug)]
pub struct MADT {
    /// Table header.
    pub header: ACPISDTHeader,
    /// 32-bit physical address at which each processor can access its local interrupt controller. 
    pub local_interrupt_ctrl_addr: u32,
    /// Multiple APIC Flags.
    pub flags: u32,
    /* 
     * Interrupt Controller Structure may vary on different systems with different hardware. 
     * Because of that, they are not defined within the structure but rather obtained by iterating
     * over each entry via ICSEntryIter. 
     * */
    __entries: (),
}

impl SystemDescriptionTable for MADT {
    const SIGNATURE: &'static str = "APIC";
}

impl MADT {
    /// Retuns an iterator over MADT entries.
    ///
    /// Different hardware may have different amount of entries, therefore they must be properly
    /// iterated and parsed based on their type id.
    pub fn entries(&mut self) -> ICSEntryIter {
        let ptr = unsafe { ptr::from_mut(&mut self.__entries) };
        ICSEntryIter {
            current: unsafe { mem::transmute(ptr) },
            limit: ptr.addr() + self.header.length as usize,
        }
    }
}

bitflags! {
    /// Multiple APIC Flags.
    ///
    /// MAF fields in MADT table is a 32-bit value, which has only one important bit that defines
    /// if system has dual 8259 PICs on the system. Other fields are reserved and are always zero.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct MAF: u32 {
        /// Bit is set whether the CPU contains dual 8259 legacy PIC's installed. If bit 0 in the
        /// flags is set, then PIC's interrupts must be masked.
        const PCAT_COMPAT = 1 << 0,
        /* Other bits must be zero. */
    };

    /// CPU's local APIC flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct LocadAPICFlags: u32 {
        /// If this bit is set the processor is ready for use. If clear but ONLINE_CAPABLE bit is
        /// set, then the hardware supports enabling the selected processor during OS runtime. If
        /// both bits are cleared, this processor is unusable.
        const ENABLED           = 1 << 0,
        /// Related to ENABLED bit above.
        const ONLINE_CAPABLE    = 1 << 1,
        /* Other bits must be zero. */
    };

    /// MPS INTI Flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct MPS_INTI_Flags: u16 {
        /// Specific to the specifications of the bus.
        const POLARITY_BUS_SPECIFIC = 0b00,
        /// Active high polarity.
        const POLARITY_ACTIVE_HIGH = 0b01,
        /* Polarity value 0b10 is reserved. */
        /// Active low polarity.
        const POLARITY_ACTIVE_LOW = 0b00,
    };
}

/// Iterator over ICS entries.
///
/// Can only be obtained from the MADT table.
pub struct ICSEntryIter<'a> {
    /// Holds a pointer to the ICS Entry.
    current: Option<&'a mut ICSEntry<'a>>,
    limit: usize,
}

impl<'a> Iterator for ICSEntryIter<'a> {
    type Item = &'a mut ICSEntry<'a>;

    fn next(&mut self) -> Option<<Self as Iterator>::Item> {
        /* As long as this statement is true, we can expect more entries forward. */
        if let Some(c) = self.current.take() {
            unsafe {
                self.current = 
                    ptr::from_mut(c)
                    .byte_add(c.header.size as usize)
                    .map_addr(|ptr| if ptr < self.limit {ptr} else {0})
                    .as_mut();
                Some(c)
            }
        } else { None }
    }
}

/// Entry types within the Interrupt Controller Structure.
#[repr(u8)]
#[derive(Iternum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ICSEntryType {
    ProcessorLocalAPIC                  = 0x0,
    IOAPIC                              = 0x1,
    InterruptSourceOverride             = 0x2,
    NMISource                           = 0x3,
    LocalAPICNMI                        = 0x4,
    LocalAPICAddressOverride            = 0x5,
    IOSAPIC                             = 0x6,
    LocalSAPIC                          = 0x7,
    PlatformInterruptSources            = 0x8,
    ProcessorLocalx2APIC                = 0x9,
    Localx2APICNMI                      = 0xa,
    GICCPUInterface                     = 0xb,
    GICDistributor                      = 0xc,
    GICMSIFrame                         = 0xd,
    GICRedistributor                    = 0xe,
    GICInterruptTranslationService      = 0xf,
    MultiprocessorWakeup                = 0x10,
    CorePIC                             = 0x11,
    LegacyPIC                           = 0x12,
    HyperTransportPIC                   = 0x13,
    ExtendedPIC                         = 0x14,
    MSIPIC                              = 0x15,
    BridgeIOPIC                         = 0x16,
    LowPinCountPIC                      = 0x17,
}

/// Interrupt Controller Structure Entry.
///
/// MADT contains up to 24 of those entries, but not necessary all of them.
#[repr(C, packed)]
pub struct ICSEntry<'a> {
    header: ICSEntryHeader,
    body: ICSEntryBody<'a>,
}

/// Interrupt Controller Structure Entry Header.
///
/// Each entry in MADT table begins with two bytes defining it's type and size.
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct ICSEntryHeader {
    /// Type of this specific ICS entry.
    r#type: ICSEntryType,
    /// Size value of the whole entry in bytes.
    size: u8,
}

/// ICS Entry Body
///
/// This enum defines all possible entry bodies that could be found within the MADT table.
#[derive(Debug)]
pub enum ICSEntryBody<'a> {
    ProcessorLocalAPIC(&'a mut ProcessorLocalAPIC),
    IOAPIC(&'a mut IOAPIC), 
    InterruptSourceOverride(&'a mut InterruptSourceOverride),
    NMISource(&'a mut NMISource),
    LocalAPICNMI(&'a mut LocalAPICNMI),
    LocalAPICAddressOverride(&'a mut LocalAPICAddressOverride),
    IOSAPIC(&'a mut IOSAPIC),
    LocalSAPIC(&'a mut LocalSAPIC),
}

/// Processor Local APIC (Entry 0)
///
/// Represents a single logical processor and it's interrupt controller. When using APIC
/// interrupt model, each processor in the system is required to have a Processor Local APIC
/// record in the MADT, and a processor device object in the DSDT.
///
/// Processors are not allowed to be added while in sleeping state.
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct ProcessorLocalAPIC {
    acpi_processor_uid: u8,
    apic_id : u8,
    flags: u32,
}
/// I/O APIC (Entry 1)
///
/// The global system interrupt base is the first interrupt number that this I/O APIC handles.
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct IOAPIC {
    io_apic_id: u8,
    _reserved: u8,
    io_apic_addr: u32,
    global_system_interrupt_base: u32,
}
/// Interrupt Source Override (Entry 2)
///
/// Allows to map interrupt sources to global system interrupts.
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct InterruptSourceOverride {
    bus: u8,
    source: u8,
    global_system_interrupt: u32,
    flags: u16,
}
/// Non-Maskable Interrrupt Source Structure (Entry 3)
///
/// Specifies whether APIC interrupts should be enabled as non-maskable. All non-maskable
/// sources will not be available for use by devices.
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct NMISource {
    flags: u16,
    global_system_interrupt: u32,
}
/// Local APIC NMI (Entry 4)
///
/// Describes local APIC interrupt input lines that NMI is connected to for each of the processors
/// in the system where such connection exist.
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct LocalAPICNMI {
    acpi_processor_uid: u8,
    flags: u16,
    local_apic_lint: u8,
}
/// Local APIC Address Override (Entry 5)
///
/// Optional structure for overriding the physical addresses of the local APIC in the MADT's
/// table header, which is defined as 32-bit field.
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct LocalAPICAddressOverride {
    _reserved: u16,
    local_apic_address: u64,
}
/// I/O SAPIC (Entry 5)
///
/// Similar to I/O APIC structure (Entry 2). If both exist, this one shall be used.
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct IOSAPIC {
    io_sapic_id: u8,
    _reserved: u8,
    global_system_interrupt_base: u32,
    io_sapic_addr: u64,
}
/// Local SAPIC (Entry 6)
///
/// Similar to local APIC (Entry 1). When using SAPIC each processor in the system is required 
/// to have a Processor Local APIC record in the MADT, and a processor device object in the DSDT.
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct LocalSAPIC {
    acpi_processor_id: u8,
    local_sapic_id: u8,
    local_sapic_eid: u8,
    _reserved: (u16, u8),
    flags: u32,
    acpi_processor_uid: u32,
    acpi_processor_uid_string: &'static [OSChar], 
}
