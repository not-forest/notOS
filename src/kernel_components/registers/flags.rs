/// Control module for x86 FLAGS, EFLAGS and RFLAGS.

use crate::bitflags;
use core::arch::asm;

/// An interface to operate on FLAGS, EFLAGS and RFLAGS combined. Basically it allows to work
/// with all three variants.
/// 
/// The FLAGS register is the status register that contains the current state of an x86 CPU. 
/// The size and meanings of the flag bits are architecture dependent. It usually reflects 
/// the result of arithmetic operations as well as information about restrictions placed on 
/// the CPU operation at the current time.
/// 
/// In the i286 architecture, the register is 16 bits wide. Its successors, the EFLAGS and 
/// RFLAGS registers, are 32 bits and 64 bits wide, respectively. The wider registers retain 
/// compatibility with their smaller predecessors.
pub struct XFLAGS;

impl XFLAGS {
    /// Reads the current flag sets of RFLAGS register.
    #[inline]
    pub fn read() -> XFLAGSFlags {
        let flags: u64;

        unsafe {
            asm!("pushfq; pop {}", out(reg) flags, options(nomem, preserves_flags));
        }

        flags.into()
    }

    /// Write a new value to RFLAGS.
    /// 
    /// # Unsafe
    /// 
    /// Undefined behavior can occur if the wrong flags are set.
    #[inline]
    pub unsafe fn write(flags: XFLAGSFlags) {
        let old_flags = Self::read();
        let reserved = old_flags & !XFLAGSFlags::all();
        let new_flags = reserved | flags;
        
        unsafe {
            asm!("mov cr0, {}", in(reg) u64::from(new_flags), options(nostack, preserves_flags));
        }
    }
}

bitflags!{
    /// Config for XFLAGS.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct XFLAGSFlags: u64 {
        // # FLAGS

        /// Carry Flag (CF) - Indicates a carry out of the most significant bit during an operation.
        const CARRY_FLAG = 1,
        // The 2nd bit is reserved.
        /// Parity Flag (PF) - Set if the number of set bits in the result is even.
        const PARITY_FLAG = 1 << 2,
        // The 4rd bit is reserved.
        /// Auxiliary Carry Flag (AF) - Used for binary-coded decimal (BCD) arithmetic.
        const AUXILIARY_CARRY_FLAG = 1 << 4,
        // the 6th bit is reserved.
        /// Zero Flag (ZF) - Set if the result of an operation is zero.
        const ZERO_FLAG = 1 << 6,
        /// Sign Flag (SF) - Set if the result of an operation is negative.
        const SIGN_FLAG = 1 << 7,
        /// Trap Flag (TF) - Used for single-step debugging.
        const TRAP_FLAG = 1 << 8,
        /// Interrupt Enable Flag (IF) - If set, interrupts are enabled.
        const INTERRUPT_FLAG = 1 << 9,
        /// Direction Flag (DF) - Affects string operations.
        const DIRECTION_FLAG = 1 << 10,
        /// Overflow Flag (OF) - Set if an arithmetic operation produces a signed overflow.
        const OVERFLOW_FLAG = 1 << 11,
        /// Input/Output Privilege Level (IOPL) - Two bits representing I/O privilege level.
        const IO_PRIVILEGE_LEVEL = 3 << 11,
        /// Nested Task Flag (NT) - Indicates that a task switch occurred during the execution of a task.
        const NESTED_TASK_FLAG = 1 << 14,
        /// Mode Flag (MF) - Indicates the processor's operating mode (protected mode or real mode).
        const MODE_FLAG = 1 << 15,

        // EFLAGS

        /// Resume Flag (RF) - Used in debugging to resume execution after a breakpoint.
        const RESUME_FLAG = 1 << 16,
        /// Virtual Mode Flag (VM) - Indicates virtual 8086 mode.
        const VIRTUAL_MODE = 1 << 17,
        /// Alignment Check Flag (AC) - Set if an alignment check exception occurs.
        const ALIGNMENT_CHECK = 1 << 18,
        /// Virtual Interrupt Flag (VIF) - Indicates virtual interrupt enable.
        const VIRTUAL_INTERRUPT_FLAG = 1 << 19,
        /// Virtual Interrupt Pending Flag (VIP) - Indicates virtual interrupt pending.
        const VIRTUAL_INTERRUPT_PENDING = 1 << 20,
        /// CPUID Enable Flag (ID) - If set, the CPUID instruction is available for use.
        const CPUID_ENABLE = 1 << 21,
        // bits 23-30 are reserved.
        /// AES Key Schedule Loaded Flag (AESKL) - Indicates whether AES key schedule is loaded.
        const AES_KEY_SCHEDULE_LOADED_FLAG = 1 << 30,
        /// Alternative Instruction Set Enable Flag (AISE) - Enables an alternative instruction set.
        const ALTERNATIVE_INSTRUCTION_SET_ENABLE = 1 << 31,

        // RFLAGS bits are reserved and not used for any inner purpose.
    }
}