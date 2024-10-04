/// Defines predefined CPU exception handler functions.

/// A collection of predefined functions that can be used within the gates.
use crate::{println, print, debug, Color, critical_section};
use super::handler_functions::*;

#[no_mangle]
unsafe extern "x86-interrupt" fn division_by_zero_handler(stack_frame: InterruptStackFrame) -> ! {
    println!(Color::RED; "EXCEPTION: Division by zero.");
    debug!("{:#?}", stack_frame);
    loop {}
}

#[no_mangle]
unsafe extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    println!(Color::RED; "EXCEPTION: Breakpoint");
    debug!("{:#?}", stack_frame);
}

#[no_mangle]
unsafe extern "x86-interrupt" fn double_fault_handler(stack_frame: InterruptStackFrame) {
    println!(Color::RED; "EXCEPTION: Double Fault");
    debug!("{:#?}", stack_frame);
    loop {}
}

#[no_mangle]
unsafe extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: ErrorCode,
) {
    critical_section!(|| {
        println!(Color::RED; "EXCEPTION: Page Fault");
        debug!("{:#?}", stack_frame);

        print!("Error code flags: ");
        for error in PageFaultErrorCode::as_array() {
            if error.is_in(error_code.0) {
                print!("{:?} ", error);
            }
        } println!();
    });
    loop {}
}

/// A regular division by zero handler. ('#DE')
/// 
/// This function provides the error info and a current stack table information.
pub const DIVISION_BY_ZERO: DivergingHandlerFunction = division_by_zero_handler;
/// Sets a breakpoint. ('#BP')
/// 
/// Will provide a current stack table information.
pub const BREAKPOINT: HandlerFunction = breakpoint_handler;

/// Double fault handler. ('#DF')
/// 
/// Double fault occur when the entry for some function is not set to the
/// corresponding interrupt vector or a second exception occurs inside the
/// handler function of the prior exception.
/// 
/// It only works for a certain combinations of exceptions:
/// - '#DE' -> '#TS', '#NP', '#SS', 'GP';
/// - '#TS' -> '#TS', '#NP', '#SS', 'GP';
/// - '#NP' -> '#TS', '#NP', '#SS', 'GP';
/// - '#SS' -> '#TS', '#NP', '#SS', 'GP';
/// - '#GP' -> '#TS', '#NP', '#SS', 'GP';
/// - '#PF' -> '#TS', '#NP', '#SS', 'GP', '#PF';
pub const DOUBLE_FAULT: HandlerFunction = double_fault_handler;

/// A page fault function handler.
/// 
/// There are many ways for the page fault to occur, therefore the error code
/// must be used accordingly as it does provide additional info about the reason
/// of the page fault invocation.
pub const PAGE_FAULT: HandlerFunctionWithErrCode = page_fault_handler;

