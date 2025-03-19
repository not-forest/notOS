/// MADT table allows to query local APICs on every CPU core and manage interrupts.

use super::acpi::{ACPISDTHeader, SystemDescriptionTable};
use crate::{bitflags, kernel_components::os::OSChar};
use core::ptr;

/// Multiple APIC Description Table
///
/// This ACPI table provides informations necessary for managing APIC, SAPIC, GIC or LPIC
/// controllers. It describes all of the interrupt controllers in the system and can be used to
/// enumerate currently available CPUs.
#[repr(C)]
#[derive(Debug)]
pub struct MADT {
    /// Table header.
    header: ACPISDTHeader,
    /// 32-bit physical address at which each processor can access its local interrupt controller. 
    local_interrupt_ctrl_addr: u32,
    /// Multiple APIC Flags.
    flags: MAF,
    /* Interrupt Controller Structure Entries. */
    processor_local_apic: ProcessorLocalAPIC,
    io_apic: IOAPIC,
    interrupt_source_override: InterruptSourceOverride,
    nmi_source: NMISource,
    local_apic_nmi: LocalAPICNMI,
}

impl SystemDescriptionTable for MADT {
    const SIGNATURE: &'static str = "APIC";
}

impl MADT {
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

/// Entry types within the Interrupt Controller Structure.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ICSEntryType {
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

/// Interrupt Controller Structure Entry Header.
///
/// Each entry in MADT table begins with two bytes defining it's type and size.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ICSEntryHeader {
    /// Type of this specific ICS entry.
    r#type: ICSEntryType,
    /// Size value of the whole entry in bytes.
    size: u8,
}

/// Processor Local APIC (Entry 0)
///
/// Represents a single logical processor and it's interrupt controller. When using APIC
/// interrupt model, each processor in the system is required to have a Processor Local APIC
/// record in the MADT, and a processor device object in the DSDT.
///
/// Processors are not allowed to be added while in sleeping state.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ProcessorLocalAPIC {
    header: ICSEntryHeader,
    acpi_processor_uid: u8,
    apic_id : u8,
    flags: LocadAPICFlags,
}

/// I/O APIC (Entry 1)
///
/// The global system interrupt base is the first interrupt number that this I/O APIC handles.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct IOAPIC {
    header: ICSEntryHeader,
    io_apic_id: u8,
    _reserved: u8,
    io_apic_addr: u32,
    global_system_interrupt_base: u32,
}

/// Interrupt Source Override (Entry 2)
///
/// Allows to map interrupt sources to global system interrupts.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct InterruptSourceOverride {
    header: ICSEntryHeader,
    bus: u8,
    source: u8,
    global_system_interrupt: u32,
    flags: MPS_INTI_Flags,
}

/// Non-Maskable Interrrupt Source Structure (Entry 3)
///
/// Specifies whether APIC interrupts should be enabled as non-maskable. All non-maskable
/// sources will not be available for use by devices.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct NMISource {
    header: ICSEntryHeader,
    flags: MPS_INTI_Flags,
    global_system_interrupt: u32,
}

/// Local APIC NMI (Entry 4)
///
/// Describes local APIC interrupt input lines that NMI is connected to for each of the processors
/// in the system where such connection exist.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct LocalAPICNMI {
    header: ICSEntryHeader,
    acpi_processor_uid: u8,
    flags: MPS_INTI_Flags,
    local_apic_lint: u8,
}

/// Local APIC Address Override (Entry 5)
///
/// Optional structure for overriding the physical addresses of the local APIC in the MADT's
/// table header, which is defined as 32-bit field.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct LocalAPICAddressOverride {
    header: ICSEntryHeader,
    _reserved: u16,
    local_apic_address: u64,
}

/// I/O SAPIC (Entry 5)
///
/// Similar to I/O APIC structure (Entry 2). If both exist, this one shall be used.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct IOSAPIC {
    header: ICSEntryHeader,
    io_sapic_id: u8,
    _reserved: u8,
    global_system_interrupt_base: u32,
    io_sapic_addr: u64,
}

/// Local SAPIC (Entry 6)
///
/// Similar to local APIC (Entry 1). When using SAPIC each processor in the system is required 
/// to have a Processor Local APIC record in the MADT, and a processor device object in the DSDT.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct LocalSAPIC {
    header: ICSEntryHeader,
    acpi_processor_id: u8,
    local_sapic_id: u8,
    local_sapic_eid: u8,
    _reserved: (u16, u8),
    flags: LocadAPICFlags,
    acpi_processor_uid: u32,
    acpi_processor_uid_string: &'static [OSChar], 
}
