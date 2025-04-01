/// Interrupt controller driver.
///
/// Such driver is a systemwide driver that shall not be reimplemented via custom implementations.

use alloc::boxed::Box;
use crate::kernel_components::drivers::Driver;
use crate::kernel_components::arch_x86_64::interrupts::{
    handler_functions::InterruptStackFrame,
    INTERRUPT_DESCRIPTOR_TABLE,
};
use crate::kernel_components::arch_x86_64::controllers::{
    pic::ChainedPics,
};

pub trait InterruptControllerDriver {
    /// Interrupt controller start function.
    ///
    /// Make sure a proper bit is set in global IDT table structure because some implementation 
    /// might require this information.
    ///
    /// # Default
    ///
    /// Does nothing on default behavior.
    unsafe fn epilogue(&mut self, stack_frame: InterruptStackFrame) {}
    /// Interrupt controller exit function.
    ///
    /// Usually used to send EOI command to the controller. Make sure a proper bit is set in global 
    /// IDT table structure because some implementation might require this information.
    ///
    /// # Default
    ///
    /// Does nothing on default behavior.
    unsafe fn prologue(&mut self, stack_frame: InterruptStackFrame) {}

    /// Maps the irq to int based on current interrupt controller configuration.
    fn irq_to_int(&self, irq: u8) -> u8;
}

impl InterruptControllerDriver for ChainedPics {
    /* Epilogue for PIC is used to deal with spurious interrupts. */
    unsafe fn epilogue(&mut self, stack_frame: InterruptStackFrame) {
        unimplemented!()
    }

    /* PIC required a proper EOI signal. */
    unsafe fn prologue(&mut self, stack_frame: InterruptStackFrame) {
        if let Some((i, _)) = INTERRUPT_DESCRIPTOR_TABLE.ints
                                                    .iter()
                                                    .enumerate()
                                                    .skip_while(|(ref i, b)| *i < 32)
                                                    .find(|(i, ref b)| **b) {
            if !self.is_spurious(i as u8) {
                self.notify_end_of_interrupt(i as u8);
            }            
        } else {
            crate::warn!("PIC Driver could not send EOI command.")
        }
    }

    fn irq_to_int(&self, irq: u8) -> u8 {
        match irq {
            0 .. 8 => self.master.offset + irq,
            8 .. 16 => self.slave.offset + irq,
            _ => panic!("Wrong IRQ value provided. Unable to map."),
        }
    }
}

/* All types below implement the InterruptControllerDriver trait. */
impl_driver!(Box<dyn InterruptControllerDriver>);
impl_driver!(ChainedPics);
