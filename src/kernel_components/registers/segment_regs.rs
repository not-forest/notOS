/// Module for working with segment registers.
/// 
/// Segmentation is the process in which the main memory of the computer is logically 
/// divided into different segments and each segment has its own base address. It is 
/// basically used to enhance the speed of execution of the computer system, so that 
/// the processor is able to fetch and execute the data from the memory easily and fast.
/// 
/// The Bus Interface Unit (BIU) contains four 16 bit special purpose registers called as 
/// Segment Registers: CS, DS, ES, SS. In 32-bit mode, two more segments are provided FS, GS.
/// In 64-bit mode the first four segments are ignored and the vase address is always 0 to
/// provide a full 64-bit address space. The FS and GS however are still usable partially.

use super::{
    control::Cr4Flags,
    ms::Msr,
};
use crate::kernel_components::arch_x86_64::PrivilegeLevel;
use crate::VirtualAddress;

/// Trait that represent the x86 segment.
/// 
/// A segment is a logically contiguous chunk of memory with consistent properties (from the CPU's perspective).
/// 
/// The segment registers in pure real-mode are limited to 16 bits for addressing and represented
/// as segment selectors. In 64-bit mode, most of the segmentation functionality is disabled.
pub trait Segment {
    /// Reads data from the provided segment.
    fn read() -> SegmentSelector;
    /// Writes data to the provided segment.
    /// 
    /// # Unsafe
    /// 
    /// Undefined behavior can occur in a various possible ways. Writes must be done carefully,
    /// because the outcome will depend on the selected segment.
    unsafe fn write(selector: SegmentSelector);
}

/// Trait that represent the x86_64 segment.
/// 
/// This type of segment can be used in 64-bit mode, even though not fully. This trait is only
/// valuable for FS and GS segments, which have a base that is 64 bit.
pub trait Segment64Bit<T>: Segment where T: Msr {
    /// A msr base which can be either fsbase, gsbase or kernelgsbase.
    const BASE: T;

    /// Reads the base address of the segment
    fn read_base() -> VirtualAddress;
    /// Writes the segment base address.
    /// 
    /// # Unsafe
    /// 
    /// Undefined behavior can occur in a various possible ways. Writes must be done carefully.
    unsafe fn write_base(addr: VirtualAddress);
}

/// A Segment Selector is a 16-bit binary data structure specific to the IA-32 and x86-64
/// architectures. It is used in Protected Mode and Long Mode.
/// 
/// A reference to a descriptor, which you can load into a segment register; the selector is
/// an offset into a descriptor table pointing to one of its entries. These entries are 
/// typically 8 bytes long, therefore bits 3 and up only declare the descriptor table entry 
/// offset, while bit 2 specifies if this selector is a GDT or LDT selector (LDT - bit set, 
/// GDT - bit cleared), and bits 0 - 1 declare the ring level that needs to correspond to 
/// the descriptor table entry's DPL field. If it doesn't, a General Protection Fault occurs;
/// if it does correspond then the CPL level of the selector used is changed accordingly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct SegmentSelector(u16);

impl SegmentSelector {
    /// Creates a new instance of 'SegmentSelector'.
    /// 
    /// ## Index
    /// 
    /// Bits 3-15 of the Index of the GDT or LDT entry referenced by the selector.
    /// Since Segment Descriptors are 8 bytes in length, the value of Index is never
    /// unaligned and contains all zeros in the lowest 3 bits.
    /// 
    /// ## TI
    /// 
    /// Specifies which descriptor table to use. If clear 0 then the GDT is used, 
    /// if set (1) then the current LDT is used.
    /// 
    /// ## RPL
    /// 
    /// The requested Privilege Level of the selector, determines if the selector is 
    /// valid during permission checks and may set execution or memory access privilege.
    #[inline]
    pub const fn new(index: u16, use_ldt: bool, level: PrivilegeLevel) -> Self {
        Self(index << 3 | (use_ldt as u16) << 2 | (level as u16))
    }

    /// Returns the GDT index.
    #[inline]
    pub const fn get_index(&self) -> u16 {
        self.0 >> 3
    }

    #[inline]
    pub const fn get_privilege_level(&self) -> u16 {
        self.0 & 0x6
    }
}


/// Code segment (CS) is used for addressing memory location in the code segment of the 
/// memory, where the executable program is stored.
#[derive(Debug)]
pub struct CodeSegment;

/// Stack segment (SS) is used for addressing stack segment of the memory. The stack 
/// segment is that segment of memory which is used to store stack data.
#[derive(Debug)]
pub struct StackSegment;

/// Data segment (DS) points to the data segment of the memory where the data is stored.
#[derive(Debug)]
pub struct DataSegment;

/// Extra segment (ES) also refers to a segment in the memory which is another data segment in the memory.
#[derive(Debug)]
pub struct ExtraSegment;

/// Function segment (FS) is undefined at a hardware level and can be used how the OS decides.
#[derive(Debug)]
pub struct FunctionSegment;

/// General segment (GS) is undefined at a hardware level and can be used how the OS decides.
#[derive(Debug)]
pub struct GeneralSegment;