/// Interrupts and other procedures to the CPU. 
/// 
/// Exception catching are done with interrupt description table and handler functions.

use core::arch::asm;

/// Enables interrupts
#[inline(always)]
pub fn enable() {
    unsafe { asm!("sti", options(preserves_flags, nostack)) }
}

/// Disables interrupts.
#[inline(always)]
pub fn disable() {
    unsafe { asm!("cli", options(preserves_flags, nostack)) }
}

/// The hlt function wrapper
#[inline(always)]
pub fn hlt() {
    unsafe { asm!("hlt", options(nomem, nostack)) }
}

/// Cause a breakpoint.
/// 
/// This is a wrapper around a regular int3 instruction.
#[inline(always)]
#[no_mangle]
pub fn breakpoint() {
    unsafe { asm!("int3", options(nomem, nostack)) }
}

/// Divides a given integer by zero.
/// 
/// This function is only usable to test out the handler function, that must be called after such
/// operation. The provided integer must be u32.
#[inline(always)]
#[no_mangle]
pub fn divide_by_zero(input: u32) {
    unsafe {
        asm!(
            "mov {0:r}, rax",            // Load the input value into RAX
            "mov rdx, 0",                // Set RDX to 0 to create a 64-bit dividend
            "div rdx",                   // Divide RAX by RDX (zero)
            "mov {0:r}, rax",            // Store the result back into the input variable
            inout(reg) input => _
        );
    }
}