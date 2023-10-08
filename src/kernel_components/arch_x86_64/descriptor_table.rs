/// This module provides useful commands for working with descriptor tables

use crate::VirtualAddress;
use core::arch::asm;

/// A pointer to a descriptor table.
/// 
/// This pointer can be obtain during "sgdt" or "sidt" or used with "lgdt" and "lidt" asm 
/// instructions. There is no need to use this structure and make assembly manually, use 
/// GDT::load_table() or IDT::load_table() methods.
/// 
/// ## Use
/// 
/// This struct has no read and write methods as it is used differently in GDT and IDT. It is
/// used within some functions of the GDT and IDT structs and they do have all of the functions
/// needed to control and manage descriptor tables 
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(C, packed(2))]
pub struct DTPointer {
    pub addr: VirtualAddress,
    pub size: u16,
}

impl DTPointer {
    /// Returns a zeroed struct.
    /// 
    /// Do not use it load functions as it would cause undefined behavior. Zeroed pointer is
    /// only needed to read data from the GDT and IDT and rewrite the pointer's bits.
    #[inline(always)]
    pub const fn null() -> Self {
        Self { addr: 0, size: 0 }
    }
}

/// Load a GDT via lgdt instruction.
///
/// ## Safety
///
/// The pointer must be valid static struct. To get the same result safely, it is better to
/// use the GDT::load_table() 
#[inline]
pub unsafe fn lgdt(gdt_ptr: &DTPointer) {
    unsafe {
        asm!("lgdt [{}]", in(reg) gdt_ptr, options(readonly, nostack, preserves_flags));
    }
}

/// Get the address of the current GDT via sgdt instruction.
/// 
/// This is always safe as it does no harm to the system in any way.
#[inline]
pub fn sgdt() -> DTPointer {
    let mut gdt_ptr = DTPointer::null();

    unsafe {
        asm!("sgdt [{}]", in(reg) &mut gdt_ptr, options(nostack, preserves_flags));
    }

    gdt_ptr
}

/// Load a IDT via lidt instruction.
///
/// ## Safety
///
/// The pointer must be valid static struct. To get the same result safely, it is better to
/// use the IDT::load_table() 
#[inline]
pub unsafe fn lidt(idt_ptr: &DTPointer) {
    unsafe {
        asm!("lidt [{}]", in(reg) idt_ptr, options(readonly, nostack, preserves_flags));
    }
}

/// Get the address of the current IDT via sidt instruction.
/// 
/// This is always safe as it does no harm to the system in any way.
#[inline]
pub fn sidt() -> DTPointer {
    let mut idt_ptr = DTPointer::null();

    unsafe {
        asm!("sidt [{}]", in(reg) &mut idt_ptr, options(nostack, preserves_flags));
    }

    idt_ptr
}