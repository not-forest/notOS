/// Module that implements FADT table and it's management.
///
/// This table contains information about fixed register blocks pertaining to power management
/// and it is mainly used for creating a proper shutdown procedure. As all other tables, RSDT is
/// required for locating one.

use crate::bitflags;
use super::acpi::{ACPISDTHeader, SystemDescriptionTable, GenericAddressStructure};
use proc_macros::public;

#[repr(C)]
#[public]
#[derive(Debug)]
pub struct FADT {
    /// Table header.
    header: ACPISDTHeader,
    /// This is a 32-bit pointer to the FACS. Since ACPI 2.0 a new field has been added to the 
    /// table, X_FirmwareControl of type GAS, which is 64-bits wide. Only one of the two fields 
    /// is used, the other contains 0. According to the Specs, the X_ field is used only when 
    /// the FACS is placed above the 4th GB.
    firmware_ctrl: u32,
    /// Physical memory address of the DSDT. If the X_DSDT field contains a non-zero value, which
    /// can be used by the OSPM, then this field must be ignored in OSPM.
    dsdt: u32,
    /// ACPI 1.0 legacy field, which is now reserved and must stay 0.
    _reserved: u8,
    /// Preffered power management mode. This field is set by the OEM to convey the preferred power
    /// management profile to OSPM.
    ppmp: PPMP,
    /// System vector the SCI interrupt is wired to in PIC 8259 mode. On systems without 8259, this
    /// field contains the global system interrupt number of the SCI interrupt.
    sci_int: u16,
    /// System port address of the SMI command port.
    smi_cp: u32,

    /// The value to write to SMI_CMD to disable SMI ownership of the ACPI hardware registers.
    apci_en: u8,
    /// The value to write to SMI_CMD to re-enable SMI ownership of the ACPI hardware registers.
    apci_dis: u8,
    /// The value to write to SMI_CMD to enter the S4BIOS state.
    s4bios_req: u8,
    /// If non-zero, this field contains the value OSPM writes to the SMI_CMD register to assume
    /// processor performance state control responsibility
    pstate_ctrl: u8,
    
    /// System port addres of the PM1a event register block. Required if X_PM1a_EVT_BLK field
    /// contains a non zero value.
    pm1a_event_block: u32,
    /// System port addres of the PM1b event register block. Required if X_PM1b_EVT_BLK field
    /// contains a non zero value.
    pm1b_event_block: u32,
    /// System port addres of the PM1a control register block. Required if X_PM1a_CNT_BLK field
    /// contains a non zero value.
    pm1a_control_block: u32,
    /// System port addres of the PM1b control register block. Required if X_PM1b_CNT_BLK field
    /// contains a non zero value.
    pm1b_control_block: u32,
    /// System port addres of the PM2 event register block. Required if X_PM2_CNT_BLK field
    /// contains a non zero value.
    pm2_control_block: u32,
    /// System port address of the power management timer control register block.
    pm_timer_block: u32,

    /// System port address of the general-purpose event 0 register block.
    gpe0_block: u32,
    /// System port address of the general-purpose event 1 register block.
    gpe1_block: u32,

    /// Number of bytes decoded by PM1a_EVT_BLK and if supported, PM1b_EVT_BLK.
    pm1_event_length: u8,
    /// Number of bytes decoded by PM1a_CNT_BLK and if supported, OM1b_CNT_BLK.
    pm1_control_length: u8,
    /// Number of bytes decoded by PM2_CNT_BLK.
    pm2_control_length: u8,
    /// Number of values decoded by PM_TMR_BLK. Must be 4 if PM timer is supported. Zero otherwise.
    pm_timer_length: u8,
   
    /// Number of values decoded by GPE0_BLK.
    gpe0_length: u8,
    /// Number of values decoded by GPE1_BLK.
    gpe1_length: u8,
    /// Offset within the ACPI general-purpose event model where GPE1 based events start.
    gpe1_base: u8,
    
    /// Contains the value of SMI_CMD register from the OSPM. Zero otherwise.
    cstate_ctrl: u8,
    /// The worst-case hardware latency, in microseconds, to enter and exit a C2 state. A value
    /// that is bigger than 100 indicated that the system does not support a C2 state.
    worst_c2_latency: u16,
    /// The worst-case hardware latency, in microseconds, to enter and exit a C3 state. A value
    /// that is bigger than 1000 indicated that the system does not support a C3 state.
    worst_c3_latency: u16,
    /// Number of flush strides that need to be read to completely flush dirty lines from any 
    /// processor's memory caches. ONLY IF WBINVD = 0.
    flush_size: u16,
    /// The value of this field is the cache line width, in bytes, of the processor's memory
    /// caches. ONLY IF WBINVD = 0
    flush_stride: u16,

    /// The zero-based index of where the processor's duty cycle setting is within the processor's
    /// P_CNT register.
    duty_offset: u8,
    /// TGe bit width of the processor's duty cycle setting allows the software to select a nominal
    /// processor frequency below its absolute frequency.
    duty_width: u8,

    /// The RTC CMOS RAM index to the day-of-month alarm value. IF SUPPORTED.
    day_alarm: u8,
    /// The RTC CMOS RAM index to the month of the year alarm value. IS SUPPORTED.
    month_alarm: u8,
    /// The RTC CMOS RAM index to the century of data value. IF SUPPORTED.
    century: u8,

    /// IA-PC boot architecture flags.
    boot_arch_flags: u16, // Reserved in ACPI 1.0
    _reserved2: u8, // Must be 0
    /// Mixed feature flags.
    flags: u32,
    /// The addres of the reset register. 
    reset_reg: GenericAddressStructure,

    /// A value to write to reset register to reset the system.
    reset_val: u8, 
    // Reserved (FADT minor version, ARM_BOOT_ARCH)
    _reserved3: [u8; 3],
    /// Extended physical address of the FACS. 
    X_FIRMWARE_CONTROL: u64,
    /// Extended physical addtess of DSDT.
    X_DSDT: u64,

    // Extended addresses of PM blocks.
    X_PM1a_EVENT_BLOCK: GenericAddressStructure,
    X_PM1b_EVENT_BLOCK: GenericAddressStructure,
    X_PM1a_CONTROL_BLOCK: GenericAddressStructure,
    X_PM1b_CONTROL_BLOCK: GenericAddressStructure,
    X_PM2_CONTROL_BLOCK: GenericAddressStructure,
    X_PM_TIMER_BLOCK: GenericAddressStructure,
    // Extended addresses of GPE blocks.
    X_GPE0_BLOCK: GenericAddressStructure,
    X_GPE1_BLOCK: GenericAddressStructure,
}

/// Preferred Power Management Profile
///
/// This field Specifies a power management profile. Based on this value
/// power management will be handled differently by the processor. OSPM
/// can use this field to set default power management policy parameters.
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum PPMP {
    UNSPECIFIED,  
    DESKTOP, 
    MOBILE, 
    WORKSTATION, 
    ENTERPRISE_SERVER,
    SOHO_SERVER,
    APLLIANCE_PC,
    PERFORMANCE_SERVER,
    TABLET,
    // values >8 are reserved
}

impl SystemDescriptionTable for FADT {
    const SIGNATURE: &'static str = "FACP";
}

bitflags! {
    /// FADT feature flags
    ///
    /// Based on those flags, different fields define the way how data will be represented
    /// in FADT table, and allow the hardware to decide the actions to perform.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct FACPFLAG: u32 {
        /// If set, signifies that the WBINVD instruction correctly flushes the processor
        /// caches, maintains memory coherency, and upon completion of the instructuin, all 
        /// caches for the current processor contain no chached data other than what OSPM 
        /// references and allows to be cached. Required for ACPI v1.0
        const WBINVD =                               1 << 0,
        /// If set, signifies that the WBINVD instruction correctly flushes the processor
        /// caches, maintains memory coherency, but does not guarantee the caches are invalidated. 
        const WBINVD_FLUSH =                         1 << 1,
        /// If set, indicated that the C1 power state is supported on all processors. 
        const PROC_C1 =                              1 << 2,
        /// If one, indicated that the C2 power state is configured to work on a uniprocessor or
        /// multiprocessor system. Zero means that it is ready to work only with uniprocessor
        const P_LVL2_UP =                            1 << 3,
        /// A zero indicated the power button is handled as a fixed feature programming model. One
        /// means that the button is handled as a control method device. If the system does not
        /// even have a power button, then this value will always be one.
        const PWR_BUTTON =                           1 << 4,
        /// A zero indicated the sleep button is handled as a fixed feature programming model. One
        /// means that the button is handled as a control method device. If the system does not
        /// even have a sleep button, then this value will always be one.
        const SLP_BUTTON =                           1 << 5,
        /// A zero indicated the RTC wake status is supported in fixed register space. One
        /// indicates the reversed behavior. 
        const FIX_RTC =                              1 << 6,
        /// Indicated whether the RTC alarm function can wake the system from the S4 state. 
        const RTC_S4 =                               1 << 7,
        /// Zero indicated that TMR_VAL is implemented as a 24-bit calue. One means it is
        /// implemented as a 32-bit value.
        const TMR_VAL_EXT =                          1 << 8,
        /// A zero indicated that the system cannot support docking. One means it can. 
        const DCK_CAP =                              1 << 9,
        /// If set, indicated that the system supports reset via FADT reset register. 
        const RESET_REG_SUP =                        1 << 10,
        /// System attribute. Indicates that the system has no internal expansion capabilities and
        /// the case is sealed.
        const SEALED_CASE =                          1 << 11,
        /// System attribute. If set indicated the system cannot detect the monitor or
        /// keyboard/mouse devices. 
        const HEADLESS =                             1 << 12,
        /// If set, indicated to OSPM that a processor native instruction must be executed after
        /// writing the SLP_TYPx register. 
        const CPU_SW_SLP =                           1 << 13,
        /// If set, indicated the platform support the PCIEXP_WAKE_STS bit in the PM1 status
        /// register and the PCIEXP_WAKE_EN bit in the PM1 enable register. MUST BE set on
        /// platforms containing chipsets that implement PCI express and support PM1 PCIEXP_WAK
        /// bits. 
        const PCI_EXP_WAK =                          1 << 14,
        /// A value of one indicated that OSPM should use a platform provided timer to drive any
        /// motonically and non-decreading counters, such as OSPM performance counter services.
        const USE_PLATFORM_CLOCK =                   1 << 15,
        /// A one indicated that the contents of the RTC_STS flag is valid when waking the system
        /// from S4 state. 
        const S4_RTC_STS_VALID =                     1 << 16,
        /// If set, indicated that the platform is compatible with remote power-on.
        const REMOTE_POWER_ON_CAPABLE =              1 << 17,
        /// A one indicated that all local APICs must be configured for cluster destination model
        /// when delivering interrupts in logical mode. 
        const FORCE_APIC_CLUSTER_MODEL =             1 << 18,
        /// A one indicated that all local xAPICs must be configured for physical destination mode.
        /// If this bit is set, interrupt delivery operation in logical destination mode is
        /// undefined. On machines that contain fewer than 8 local xAPICs this bit is ignored.
        const FORCE_APIC_PHYSICAL_DESTINATION_MODE = 1 << 19,
        /// One indicated that the hardware-reduced ACPI is implemented. 
        const HW_REDUCED_ACPI =                      1 << 20,
        /// If set, informs OSPM that the platform is able to achieve power savinds in S0 similar
        /// to or metter than those typically achieved in S3.
        const LOW_POWER_S0_IDLE_CAPABLE =            1 << 21,
        // Rest of fields are reserved.
    }
}
