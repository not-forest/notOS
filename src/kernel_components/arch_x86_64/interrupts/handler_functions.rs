/// Module for defining the exception/interrupt handler functions and their types.

use crate::kernel_components::arch_x86_64::segmentation::{SegmentDescriptor, SegmentSelector};
use crate::kernel_components::arch_x86_64::descriptor_table::DescriptorTableType;
use crate::{VirtualAddress, bitflags};
use core::ops::{Deref, DerefMut};
use core::arch::asm;
use core::mem;

use super::INTERRUPT_DESCRIPTOR_TABLE;


/// Must be inserted in the start of any handler function that shall be flagged. Can be used in 
/// both exceptions and interrupts. Allows to halt certain tasks until the interrupt is caused.
///
/// This does the following:
/// - marks a corresponding isr bit within the IDT table.
#[macro_export]
macro_rules! handler_function_prologue {
    ($isr:expr) => {
        unsafe {
            INTERRUPT_DESCRIPTOR_TABLE
                .with_int($isr, |bit| *bit = true);
        }
    };
}

/// A marker trait for all handler functions.
pub trait HandlerFn: Sized + 'static {
    /// Gets the virtual address of the handler function. Used to load in IDT.
    fn get_virtual_addr(self) -> VirtualAddress;
}

#[doc(hidden)]
macro_rules! implement_handler_type {
    ($fun:ty) => {
        impl HandlerFn for $fun {
            #[inline]
            fn get_virtual_addr(self) -> VirtualAddress {
                self as VirtualAddress
            }
        }
    };
}

/// A regular handler function.
/// 
/// This handler function does not return any error, nor output, nor it diverges.
pub type HandlerFunction = unsafe extern "x86-interrupt" fn(
    stack_frame: InterruptStackFrame
);
implement_handler_type!(HandlerFunction);

/// A handler function that "returns" some selector error code or page fault error code.
/// 
/// This function does not diverge and must always return some error code. 
pub type HandlerFunctionWithErrCode = unsafe extern "x86-interrupt" fn(
    stack_frame: InterruptStackFrame, 
    error_code: ErrorCode
);
implement_handler_type!(HandlerFunctionWithErrCode);

/// A diverging handler function.
/// 
/// A function that should never return (diverges). Usable for machine exceptions that will
/// have no way out.
pub type DivergingHandlerFunction = unsafe extern "x86-interrupt" fn(
    stack_frame: InterruptStackFrame
) -> !;
implement_handler_type!(DivergingHandlerFunction);

/// A diverging handler function that is also pushes some error into the scope.
/// 
/// This function must not return anything. Usable for machine exceptions that will
/// have no way out.
pub type DivergingHandlerFunctionWithErrCode = unsafe extern "x86-interrupt" fn(
    stack_frame: InterruptStackFrame, 
    error_code: ErrorCode
) -> !;
implement_handler_type!(DivergingHandlerFunctionWithErrCode);

/// Represents the interrupt stack frame pushed by the CPU on interrupt or exception entry.
/// 
/// This type must be used by the "x86-interrupt" calling convention.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct InterruptStackFrame {
    /// An instruction pointer to the current instruction address that must be executed.  
    pub instruction_pointer: usize,
    /// The selector of a code segment.
    pub code_segment: SegmentSelector,
    /// A values of RFLAGS register, on the moment of calling the handler function.
    pub cpu_flags: u64,
    /// Stack pointer value at the moment of the interrupt.
    pub stack_ptr: usize,
    /// The segment descriptor of the stack segment.
    /// 
    /// Only the first half of the descriptor is needed. In Long Mode usually not used.
    pub stack_segment: u64,
}

/// A representation for any error code that can be used in any handler function that has
/// an error code.
/// 
/// It is important to know which type of error code is being used, when converting to the
/// corresponding struct.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct ErrorCode(pub u64);

impl ErrorCode {
    pub const fn selector_error_code(&self) -> SelectorErrorCode {
        SelectorErrorCode::Custom(self.0)
    }

    pub const fn page_fault_error_code(&self) -> PageFaultErrorCode {
        PageFaultErrorCode::Custom(self.0)
    }
}

bitflags! {
    /// Describes an error code that must reference a segment selector, that is related to
    /// that error.
    /// 
    /// The bits 16-63 are reserved, so it is better to use new() method, if you wish to create
    /// your own error code.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct SelectorErrorCode: u64 {
        /// When set, the exception originated externally to the processor.
        const EXTERNAL_BIT = 1,
        /// Those bits are providing some info about descriptor table, that the index is
        /// referencing to.
        const DESCRIPTOR_TABLE = 0x6,
        /// Provides the index in the GDT, IDT or LDT.
        const SELECTOR_INDEX = 0xFFF8,
    };

    /// An error code related to pages.
    /// 
    /// A page fault occurs when:
    /// - a page directory or table entry is not present in physical memory;
    /// - attemptiong to load the TLB instruction with a translation for a non-executable
    /// page;
    /// - a protection check failed;
    /// - any reserved bit in the page directory or table entries is set to 1.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct PageFaultErrorCode: u64 {
        /// If this bit is set, the page fault was caused by a page protection violation.
        /// If not set, it was caused by a non-present page.
        const PRESENT_BIT =                 1,
        /// When set, he page fault was caused by a write access. When not set, it was caused
        /// by a read access.
        const WRITE_BIT =                   1 << 1,
        /// When set, the page fault was caused while CPL was equal to 3. However it does not
        /// mean that the page fault was a privilege violation, but only marks that as an
        /// opportunity.
        const USER_BIT =                    1 << 2,
        /// When set, one or more page directory entries contain reserved bits which are set to 1. 
        /// This only applies when the PSE or PAE flags in CR4 are set to 1.
        const RESERVED_WRITE =              1 << 3,
        /// If set, the page fault was caused by an instruction fetch. This only applies when
        /// the No-Execute bit is enabled.
        const INSTRUCTION_FETCH =           1 << 4,
        /// When set, the page fault was caused by a protection-key violation.
        const PROTECTION_KEY =              1 << 5,
        /// When set, the page fault was caused by a shadow stack access.
        const SHADOW_STACK =                1 << 6,
        // Bits 7 - 14 are reserved.
        /// When set, the fault was due to SGX violation.
        /// 
        /// Intel Software Guard Extensions (SGX) is a set of instruction codes implementing 
        /// trusted execution environment that are built into some Intel central processing units 
        /// CPUs. They allow user-level and operating system code to define protected private 
        /// regions of memory, called enclaves.alloc
        /// 
        /// The fault is unrelated to ordinary paging.
        const SOFTWARE_GUARD_EXTENSIONS =   1 << 15,
        // Bits 16 - 63 are reserved.
    };
}

impl SelectorErrorCode {
    /// Creates a new error code, based on the error code value.
    /// 
    /// # Panics
    /// 
    /// This function will panic, if any of the reserved bits are used in the provided value.
    #[inline]
    pub const fn new(value: u64) -> Self {
        assert!(value <= u16::MAX as u64, "No reserved bits must be used in segment error codes.");
        Self::Custom(value)
    }

    /// Creates a new error code, based on the provided error code value, dropping any
    /// reserved bits by setting them to zero.
    #[inline]
    pub const fn new_ignore(value: u64) -> Self {
        SelectorErrorCode::Custom((value as u16) as u64)
    }

    /// Checks if the error exception occurred via some external event.
    /// 
    /// Returns the value of the external bit as a bool.
    #[inline]
    pub fn is_external(&self) -> bool {
        SelectorErrorCode::EXTERNAL_BIT.is_in(self.bits())
    }

    /// Returns the descriptor table type.
    #[inline]
    pub const fn table_type(&self) -> DescriptorTableType {
        use DescriptorTableType::*;
        let value = self.get_selected_bits(
            SelectorErrorCode::DESCRIPTOR_TABLE.bits()
        ) >> 1;

        match value {
            // Only two bits are used, so we will never reach the other values.
            0b00 => Gdt,
            0b01 => Idt,
            0b10 => Ldt,
            0b11 => Idt,
            _ => unreachable!(),
        }
    }

    #[inline]
    pub const fn get_index(&self) -> u64 {
        self.get_selected_bits(
            SelectorErrorCode::SELECTOR_INDEX.bits()
        ) >> 3
    }

    /// Checks if the returned code is null.
    #[inline]
    pub const fn is_null(&self) -> bool {
        self.bits() == 0
    }
}
