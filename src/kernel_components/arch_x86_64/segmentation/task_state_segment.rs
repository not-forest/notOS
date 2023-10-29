/// Module for Task State Segment implementation.
/// 
/// # Warn
/// 
/// Task state segment is called so, because in 32bit protected mode, it contains a program's
/// state. This implementation is for 64-bit version, therefore it is used as a buffer for IDT.

use crate::VirtualAddress;
use crate::kernel_components::arch_x86_64::segmentation::SegmentSelector;
use core::arch::asm;
use core::mem;

pub const TSS_SIZE: u32 = mem::size_of::<TSS>() as u32;

/// A struct representing the Task State Segment.
/// 
/// TSS is a binary data structure specific to the IA-32 and x86-64 architectures. It holds
/// information about a task. 
/// 
/// ## Protected Mode 32-bit (NOT IMPLEMENTED FOR THIS SYSTEM).
/// 
/// In Protected Mode the TSS is primarily suited for Hardware Task Switching, where each 
/// individual Task has its own TSS. For use in software multitasking, one or two are also 
/// generally used, as they allow for entering Ring 0 code after an interrupt. 
/// 
/// ## Long Mode 64-bit.
/// 
/// In Long Mode, the TSS has a separate structure and is used to change the Stack Pointer 
/// after an interrupt or permission level change. You'll have to update the TSS yourself 
/// in the multitasking function, as it apparently does not save registers automatically.
/// 
/// # Structure
/// 
/// The structure consists of: 
/// - three stack pointers used to load the stack when a privilege level change occurs from
/// a lover level to a higher one.
/// - seven stack pointers used to load the stack when an entry in IDT has a IST value other
/// than 0.
/// - I/O map base address field. Contains a 16-bit offset from the base of the TSS to the
/// I/O Permission Bit Map.
/// - Reserved fields that must take some place in memory. 
#[derive(Debug, Clone, Copy)]
#[repr(C, packed(4))]
pub struct TSS {
    _reserved_1:                                        u32,
    pub privilege_stack_pointers_table: [VirtualAddress; 3],
    _reserved_2:                                        u64,
    pub interrupt_stack_pointers_table: [VirtualAddress; 7],
    _reserved_3:                                        u64,
    _reserved_4:                                        u16,
    pub io_map_base_address_field:                      u16,
}

impl TSS {
    /// Returns a new TSS with zeroed interrupt and privilege tables.
    #[inline]
    pub const fn new() -> Self {
        Self {
            _reserved_1:                                           0,
            privilege_stack_pointers_table:                   [0; 3],
            _reserved_2:                                           0,
            interrupt_stack_pointers_table:                   [0; 7],
            _reserved_3:                                           0,
            _reserved_4:                                           0,
            io_map_base_address_field: TSS_SIZE as u16,
        }
    }

    /// Loads the task state segment selectors value into the TSS register (TR).
    /// 
    /// This operation marks the TSS segment in GDT as busy. This prevents other
    /// CPU from getting the old data from the register. This is not usable though,
    /// because usually a single TSS segment per CPU is being used.
    /// 
    /// # Unsafe
    /// 
    /// This function is unsafe, because a right segment selector must be used.
    #[inline]
    pub unsafe fn write(selector: SegmentSelector) {
        unsafe {
            asm!("ltr {0:x}", in(reg) selector.0, options(nostack, preserves_flags));
        }
    }

    /// Reads the current segment selector from the TSS register (TR)
    #[inline]
    pub fn read() -> SegmentSelector {
        let segment: u16;

        unsafe {
            asm!("rtr {0:x}", out(reg) segment, options(nomem, nostack, preserves_flags));
        }

        SegmentSelector(segment)
    }
}