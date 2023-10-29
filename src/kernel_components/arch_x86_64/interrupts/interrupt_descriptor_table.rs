/// Module for IDT management.

use crate::kernel_components::sync::Mutex;
use crate::kernel_components::registers::segment_regs::{Segment, CodeSegment};
use crate::kernel_components::arch_x86_64::{
    segmentation::SegmentSelector,
    PrivilegeLevel,
    DTPointer,
    descriptor_table::{lidt, sidt},
};
use crate::{bitflags, single, VirtualAddress};
use super::{
    handler_functions::predefined::*,
    HandlerFn,
};
use core::marker::PhantomData;
use core::ops::Index;
use core::mem;

/// A static instance of a global IDT.
/// 
/// The table itself is a static mutable and is not hidden behind any synchronization primitive,
/// therefore it must be used with caution.
single! {
    pub mut INTERRUPT_DESCRIPTOR_TABLE: IDT = IDT::new_empty();
}

/// Interrupt description table.
/// 
/// Special table that specifies a handler function for each CPU exception. It is a binary 
/// data structure specific to the IA-32 and x86-64 architectures. It is the Protected Mode 
/// and Long Mode counterpart to the Real Mode Interrupt Vector Table (IVT) telling the CPU 
/// where the Interrupt Service Routines (ISR) are located (one per interrupt vector).
/// 
/// In theory the table can hold up to 256 handler functions, while in reality the first 32 
/// (0x0..0x1F inclusive) entries are reserved by the CPU, so called processor-generated exceptions.
/// Not only that, but some entries that go afterwards are also reserved, which leads to less space
/// for interrupts, but it is completely more than enough for a regular OS to handle.
#[derive(Debug)]
#[repr(C, align(16))]
pub struct IDT {
    table: [GateDescriptor; 256],
}

impl IDT {
    /// Creates a new empty instance of IDT.
    /// 
    /// # Length
    /// 
    /// Each entry will be marked as empty entry. All of them will be invalid, so one must manually
    /// config all standard gates. This option provides the most of possibilities and overall flexibility.
    /// Because of that, the length is zero after the initialization.
    #[inline]
    pub fn new_empty() -> Self {
        Self { table: [GateDescriptor::EMPTY; 256] }
    }
    
    /// Pushes the value of a new gate to the table.
    /// 
    /// By pushing, it just means rewriting the empty entries as a new ones.
    #[inline]
    pub fn push(&mut self, index: usize, gate: GateDescriptor) {
        assert!(index < 256, "Index is out of bounds.");

        self.table[index] = gate;
    }

    /// Returns the current table as a 'DTPointer'.
    #[inline]
    pub fn as_dt_ptr(&self) -> DTPointer {
        DTPointer {
            addr: self as *const _ as u64,
            size: (mem::size_of::<Self>() - 1) as u16,
        }
    }

    /// Returns the current table from the 'DTPointer'.
    #[inline]
    pub fn from_dt_ptr(dt_ptr: DTPointer) -> Option<&'static IDT> {
        unsafe { (dt_ptr.addr as *const Self).as_ref() }
    }

    /// Loads the table into the system.
    /// 
    /// Static lifetime guarantees the safety loading.
    #[inline]
    pub fn load_table(&'static self) {
        unsafe { lidt(&self.as_dt_ptr()) }
    }

    /// Reads the current table value from the CPU.
    #[inline]
    pub fn get_current_table() -> DTPointer {
        sidt()
    }

    /// Returns the address of the IDT structure.
    pub fn addr(&'static self) -> usize {
        self as *const IDT as usize
    }
}

impl Index<usize> for IDT {
    type Output = GateDescriptor;

    fn index(&self, index: usize) -> &Self::Output {
        &self.table[index]
    }
}

/// A 128-bit interrupt gate structure.
/// 
/// Each entry in the interrupt table is a gate. Each gate has a complex structure:
/// 
/// - Offset: A 64-bit value, split in three parts. It represents the address of the entry 
/// point of the Interrupt Service Routine.
/// - Selector: A Segment Selector with multiple fields which must point to a valid code 
/// segment in GDT.
/// - IST: A 3-bit value which is an offset into the Interrupt Stack Table, which is stored
/// in the Task State Segment. If the bits are all set to zero, the Interrupt Stack Table is 
/// not used.
/// - Type attributes is a one byte structure, which contain this:
/// -- Gate Type: A 4-bit value which defines the type of gate this Interrupt Descriptor 
/// represents. 
/// In long mode there are two valid type values: 
///     0b1110 or 0xE: 64-bit Interrupt Gate
///     0b1111 or 0xF: 64-bit Trap Gate
/// -- DPL: A 2-bit value which defines the CPU Privilege Levels which are allowed to access 
/// this interrupt via the INT instruction. Hardware interrupts ignore this mechanism.
/// -- P: Present bit. Must be set to 1 if the entry is valid. If this bit is 0, the CPU will
/// raise a double fault.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct GateDescriptor {
    offset_1: u16,                       // offset bits 0..15
    selector: SegmentSelector,           // code segment selector in GDT.
    interupt_stack_table: u8,            // first three bits contain an offset to IST. other are reserved.
    pub type_attributes: TypeAttributes, // gate type, dpl and present bit.  
    offset_2: u16,                       // offset bits 16-31
    offset_3: u32,                       // offset bits 32-63
    _reserved: u32,                      // reserved values.
}

impl GateDescriptor {
    /// An empty gate. This gate have no handler function, nor it is marked valid.
    /// 
    /// It must be used only to fill the unused entries in the table.
    pub const EMPTY: Self = Self::empty();
    pub const DIVISION_ERROR: Self = Self::empty();

    /// Returns the numerical value of the gate.
    #[inline]
    pub const fn as_u128(&self) -> u128 {
        unsafe { *(self as *const Self as *const u128) }
    }

    /// Returns an empty Gate.
    #[inline]
    pub const fn empty() -> Self {
        Self { 
            offset_1: 0, 
            selector: SegmentSelector::new(
                0, 
                false, 
                PrivilegeLevel::KernelLevel
            ), 
            interupt_stack_table: 0, 
            type_attributes: TypeAttributes::new(
                GateType::Interrupt, 
                PrivilegeLevel::KernelLevel, 
                false,
            ), 
            offset_2: 0, 
            offset_3: 0, 
            _reserved: 0, 
        }
    }

    /// Creates a new Gate for 64-bit Long Mode.
    /// 
    /// Handler function will be the function used in if the current Gate will be used during
    /// the interrupt or trap.
    #[inline]
    pub fn new<F>(
        handler_fn: F,
        selector: SegmentSelector,
        interrupt_stack_table_offset: u8,
        gate_type: GateType,
        privilege_level: PrivilegeLevel,
        is_valid: bool,
    ) -> Self where F: HandlerFn {
        let attributes = TypeAttributes::new(gate_type, privilege_level, is_valid);
        let offset = handler_fn.get_virtual_addr();

        let offset_1 = offset as u16;
        let offset_2 = (offset >> 16) as u16;
        let offset_3 = (offset >> 32) as u32;

        Self {
            offset_1: offset_1,
            selector: selector,
            interupt_stack_table: interrupt_stack_table_offset,
            type_attributes: attributes,
            offset_2: offset_2,
            offset_3: offset_3,
            _reserved: 0,
        }
    }

    /// Creates a new interrupt gate.
    /// 
    /// For more flexibility, use new() method instead. This function use some regular options
    /// for the interrupts, and takes the handler function as the input.
    #[inline]
    pub fn new_interrupt<F>(
        handler_fn: F,
    ) -> Self where F: HandlerFn {
        Self::new(
            handler_fn, 
            CodeSegment::read(), 
            0, 
            GateType::Interrupt, 
            PrivilegeLevel::KernelLevel, 
            true
        )
    }

    /// Creates a new trap gate.
    /// 
    /// For more flexibility, use new() method instead. This function use some regular options
    /// for the traps, and takes the handler function as the input.
    #[inline]
    pub fn new_trap<F>(
        handler_fn: F
    ) -> Self where F: HandlerFn {
        Self::new(
            handler_fn, 
            CodeSegment::read(),
            0, 
            GateType::Trap, 
            PrivilegeLevel::KernelLevel, 
            true
        )
    }

    /// Returns the virtual address of this IDT entry's handler function.
    #[inline]
    pub fn handler_addr(&self) -> VirtualAddress {
        let addr = self.offset_1 as usize
                     | (self.offset_2 as usize) << 16
                     | (self.offset_3 as usize) << 32;
        addr
    }
}

/// Interrupt gate types. In Long Mode only two types available.
#[repr(u8)]
pub enum GateType {
    /// Interrupt gate is need to handle an interrupt that occur. Usually it is some kind of error.
    /// 
    /// For both Protected and Long Mode the value is the same.
    Interrupt = 0xE,
    /// Trap gate is used to handle a CPU exceptions. Interrupts are disabled when such gate is used.
    /// 
    /// For both Protected and Long Mode the value is the same.
    Trap = 0xF,
    /// A task gate is used for hardware task switching. 
    /// 
    /// # WARN Protected mode only (32-bit).
    /// 
    /// For a Task Gate the Selector value should refer to a position in the GDT which specifies 
    /// a Task State Segment rather than a code segment, and the Offset value is unused and 
    /// should be set to zero. Rather than jumping to a service routine, when the CPU processes 
    /// this interrupt, it will perform a hardware task switch to the specified task. A pointer 
    /// back to the task which was interrupted will be stored in the Task Link field in the TSS.
    Task = 0x5,
    /// 16-bit interrupt gate.
    Interrupt16 = 0x6,
    /// 16-bit trao gate.
    Trap16 = 0x7
}

/// The attributes of the gate.
/// 
/// 8-bit value that contain important information about the gate.
/// - Gate Type: A 4-bit value which defines the type of gate this Interrupt Descriptor 
/// represents. 
/// In long mode there are two valid type values: 
///     0b1110 or 0xE: 64-bit Interrupt Gate
///     0b1111 or 0xF: 64-bit Trap Gate
/// - DPL: A 2-bit value which defines the CPU Privilege Levels which are allowed to access 
/// this interrupt via the INT instruction. Hardware interrupts ignore this mechanism.
/// - P: Present bit. Must be set to 1 if the entry is valid. If this bit is 0, the CPU will
/// raise a double fault.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TypeAttributes(pub u8);

bitflags!{
    /// Config for type attributes.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct TypeAttributesFlags: u8 {
        /// Type of the gate. The gate can be either an Interrupt Gate or Trap Gate in Long Mode.
        const GATE_TYPE =                   0xF,
        /// A privilege level of the segment. The CPL must be the same or bigger than this level 
        /// to be able to manipulate with the segment. 
        const DESCRIPTOR_PRIVILEGE_LEVEL =  0x60,
        /// Must be set to 1 if the entry is valid. If this bit is 0, the CPU will raise a double fault.
        const PRESENT_BIT =                 0x80,
    } 
}

impl TypeAttributes {
    /// Creates a new descriptor type attribute.
    #[inline]
    pub const fn new(
        gate_type: GateType,
        privilege_level: PrivilegeLevel,
        is_valid: bool,
    ) -> Self {
        let gt = gate_type as u8;
        let dpl = (privilege_level as u8) << 5;
        let present = (is_valid as u8) << 7;

        TypeAttributes(gt | dpl | present)
    }

    /// Creates a new type attribute for an interrupt.
    /// 
    /// The privilege level is automatically set to kernel, as only kernel must be able to control
    /// interrupts (in most systems).
    #[inline]
    pub const fn new_interrupt(is_valid: bool) -> Self {
        Self::new(
            GateType::Interrupt,
            PrivilegeLevel::KernelLevel,
            is_valid
        )
    }

    /// Creates a new type attribute for a trap descriptor.
    /// 
    /// The privilege level is automatically set to kernel, as only kernel must be able to make trap
    /// operations. For example kernel is in charge to use traps when user wants to make any memory
    /// related operations in the system.
    #[inline]
    pub const fn new_trap(is_valid: bool) -> Self {
        Self::new(
            GateType::Trap,
            PrivilegeLevel::KernelLevel,
            is_valid
        )
    }

    /// Sets the present bit on.
    /// 
    /// After this function, no more further modifications must used, because the descriptor will
    /// be "public" afterwards.
    #[inline]
    pub fn set_present(&mut self) -> &mut Self {
        self.0 |= TypeAttributesFlags::PRESENT_BIT.bits();
        self
    }

    /// Sets the different gate type.
    /// 
    /// In Long Mode the valid values can be either Interrupt or Trap.
    #[inline]
    pub fn set_gate_type(&mut self, gate_type: GateType) -> &mut Self {
        self.0 = (self.0 & !TypeAttributesFlags::GATE_TYPE.bits()) | (gate_type as u8);
        self
    }

    pub fn set_privilege_level(&mut self, privilege_level: PrivilegeLevel) -> &mut Self {
        self.0 = (self.0 & !TypeAttributesFlags::DESCRIPTOR_PRIVILEGE_LEVEL.bits()) | (privilege_level as u8);
        self
    }

    /// Sets the present bit off.
    /// 
    /// # Unsafe
    /// 
    /// This behavior can be undefined. There isn't really a need for this kind of operation, because
    /// the present bit only allows the hardware to use the descriptor.
    #[inline]
    pub unsafe fn unset_present(&mut self) -> &mut Self {
        self.0 = !self.0 & !TypeAttributesFlags::PRESENT_BIT.bits();
        self
    }
}