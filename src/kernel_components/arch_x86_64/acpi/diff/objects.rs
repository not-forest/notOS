/// This module defines different ACPI object that could be found within the namespace. Those
/// objects are mapped to variable names by the namespace and being used to perform ACPI related
/// tasks by ACPI interpreter.

use alloc::{string, vec::Vec};
use crate::kernel_components::os::UChar;

/// AML Boolean type.
pub(crate) type Boolean = bool;
/// AML String type.
pub(crate) type String = string::String;
/// AML Buffer type. (Array of raw bytes)
pub(crate) type Buffer = Vec<u8>;
/// AML Package type. (Array of AML objects)
pub(crate) type Package = Vec<ACPIObject>;
/// AML Method type. (Array of raw AML code)
pub(crate) type Method = Vec<u8>;

/// AML defined namespace object entry.
#[derive(Debug, Clone)]
pub struct ACPIObject {

}

/// ACPI AML data types.
///
/// This enum defines all data existing data types within AML language for interpreter to use.
#[derive(Debug, Clone)]
pub(crate) enum ACPIType {
    // Primitives
    Boolean(Boolean),
    Integer(Integer),
    String(String),
    Buffer(Buffer),
    Package(Package),
    Method(Method),
    
    OperationRegion(OperationRegion),
    Mutex(Mutex),
}

/// AML Integer Type
///
/// It can be either 64-bit or 32-bit integer. The size depends on the ComplianceRevision field in
/// the DefinitionBlock within AML code.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Integer {
    Bit32(u32), // ComplianceRevision < 1
    Bit64(u64), // ComplianceRevision >= 2
}

/// AML Mutex type.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Mutex {
    sync_level: u8
}

/// Operation Region
///
/// Defines a named object of a certain type (such as SystemMemory, SystemIO, PCIConfig, etc.), and
/// gives the starting address and length.
#[derive(Debug, Clone, Copy)]
pub(crate) struct OperationRegion {
    space: RegionSpace,
    offset: Integer,
    length: Integer,
}

/// Region Space
///
/// 
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RegionSpace {
    SystemMemory,
    SystemIo,
    PciConfig,
    EmbeddedControl,
    SMBus,
    SystemCmos,
    PciBarTarget,
    IPMI,
    GeneralPurposeIo,
    GenericSerialBus,
    OemDefined(u8),
}
