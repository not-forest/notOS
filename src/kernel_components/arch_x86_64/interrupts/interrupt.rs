/// Interrupts and other procedures to the CPU. 
/// 
/// Exception catching are done with interrupt description table and handler
/// functions.

use core::arch::asm;

// Enable interrupts
#[inline(always)]
pub fn enable() {
    unsafe { asm!("sti", options(nomem, nostack)) }
}

// Disable interrupts
#[inline(always)]
pub fn disable() {
    unsafe { asm!("cli", options(nomem, nostack)) }
}

// The hlt function wrapper
#[inline(always)]
pub fn hlt() {
    unsafe { asm!("hlt", options(nomem, nostack)) }
}

// Cause a breakpoint
#[inline(always)]
pub fn int3() {
    unsafe { asm!("int3", options(nomem, nostack)) }
}