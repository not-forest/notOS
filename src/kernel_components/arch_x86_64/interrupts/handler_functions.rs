/// Module for defining the handler functions and their types.

use crate::kernel_components::arch_x86_64::segmentation::{SegmentDescriptor, SegmentSelector};
use crate::kernel_components::arch_x86_64::descriptor_table::DescriptorTableType;
use crate::{VirtualAddress, bitflags};
use core::ops::{Deref, DerefMut};
use core::arch::asm;
use core::mem;

/// A marker trait for all handler functions.
pub trait HandlerFn: Sized + 'static {
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

/// A collection of predefined functions that can be used within the gates.
pub mod predefined {
    use crate::{println, print, debug, Color};
    use crate::kernel_components::arch_x86_64::interrupts;
    use super::*;

    #[no_mangle]
    unsafe extern "x86-interrupt" fn division_by_zero_handler(stack_frame: InterruptStackFrame) -> ! {
        println!(Color::RED; "EXCEPTION: Division by zero.");
        debug!(stack_frame);
        loop {}
    }

    #[no_mangle]
    unsafe extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
        println!(Color::RED; "EXCEPTION: Breakpoint");
        debug!(stack_frame);
    }

    #[no_mangle]
    unsafe extern "x86-interrupt" fn double_fault_handler(stack_frame: InterruptStackFrame) {
        println!(Color::RED; "EXCEPTION: Double Fault");
        debug!(stack_frame);
        loop {}
    }

    #[no_mangle]
    unsafe extern "x86-interrupt" fn page_fault_handler(
        stack_frame: InterruptStackFrame,
        error_code: ErrorCode,
    ) {
        interrupts::with_int_disabled(|| {
            println!(Color::RED; "EXCEPTION: Page Fault");
            debug!(stack_frame);
    
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
}

/// A collection of predefined software interrupts, that must be used with PIC or APIC
/// interrupt controller.
/// 
/// # Warn
/// 
/// For any of this function, the interrupt controller must be reprogrammed to the desired
/// interrupt vector. Every handler function entry must be 
pub mod software {
    use alloc::boxed::Box;
    use core::any::Any;

    use crate::kernel_components::arch_x86_64::interrupts::{self, interrupt};
    use crate::kernel_components::memory::EntryFlags;
    use crate::kernel_components::task_virtualization::{Thread, Scheduler, PROCESS_MANAGEMENT_UNIT, PRIORITY_SCHEDULER, ROUND_ROBIN, ThreadState};
    use crate::{println, print, debug, Color};
    use crate::kernel_components::arch_x86_64::controllers::{
        PROGRAMMABLE_INTERRUPT_CONTROLLER,
        PS2,
    };
    use super::*;
   
    /// Software timer interrupt handler
    ///
    /// This handler calls the task switch function, which allows the PMU to perform the task
    /// switching.
    #[no_mangle]
    unsafe extern "x86-interrupt" fn timer_interrupt_handler(mut stack_frame: InterruptStackFrame) {
        use crate::kernel_components::task_virtualization::{
            Scheduler, ROUND_ROBIN, PRIORITY_SCHEDULER, 

            PROCESS_MANAGEMENT_UNIT, 

            ThreadFn, Thread,
            ThreadState, ProcState,
        };

        // This thread input must be changed when the function call must be done.
        //
        // The input is a mutable reference, therefore it will be putted within the 'rdi' register.
        let mut thread_input = 0;

        // Pushing new process if it exist.
        PROCESS_MANAGEMENT_UNIT.dequeue();

        // Perform a task switch.
        //
        // With disabled software interrupts, getting a next thread from the scheduler
        // and changing both instruction and stack pointers to the corresponding pointers
        // of that thread.
        interrupts::with_int_disabled(|| {
            // Trying to obtain some tasks from a scheduler if some.
            if let Some(task) = ROUND_ROBIN.schedule() {
                // println!("{:#x?}", task);
                // Trying to find the process by task's pid.
                //
                // If not exists, we can easily delete all tasks with this pid.
                if let Some(process) = PROCESS_MANAGEMENT_UNIT.process_list
                    .lock()
                    .get_mut(task.pid) {

                    // Changing the process' stack top based on the current stack pointer.
                    process.stack.top = stack_frame.stack_ptr;
                    
                    // Trying to find the thread by task's tid.
                    //
                    // If not exists, we can delete this specific task.
                    if let Some(thread) = process.find_thread_mut(task.tid) {
                        // Getting the current instruction pointer and stack pointer.
                        let new_stack = thread.stack_ptr;
                        let new_ip  = thread.instruction_ptr;

                        // Writting the old values to the thread.
                        thread.stack_ptr = stack_frame.stack_ptr;
                        thread.instruction_ptr = stack_frame.instruction_pointer;
                        
                        // Changing the current stack pointer to the thread's ones.
                        stack_frame.stack_ptr = new_stack;
                        stack_frame.instruction_pointer = new_ip;
                       
                        match thread.thread_state {
                            ThreadState::INIT => {
                                // If the thread is only about to run we must provide some additional information for it.
                                //
                                // This basically means that we must perform the fast-call calling convention manually, so the
                                // thread can use a mutable reference to itself and perform the instructions.
                                thread_input = thread as *const _ as usize;
                                // Changing the state to running, which will not affect the thread's input.
                                thread.thread_state = ThreadState::RUNNING;

                                // Changing the thread state for the join handle.
                                if let Some(o) = &mut thread.output {
                                    o.change_state(ThreadState::RUNNING);
                                }
                            },
                            _ => (),
                        } 
                    } else {
                        // If there are no underlying threads we must delete the hangling task
                        ROUND_ROBIN.delete(*task);
                    }
                } else {
                    // If there are no underlying process, we must delete the hangling task
                    ROUND_ROBIN.delete(*task);
                }
            }    
        });

        PROGRAMMABLE_INTERRUPT_CONTROLLER.lock().master.end_of_interrupt();

        // Before the iretq instruction is done, we must change the rdi, so it can be used as
        // a pointer parameter for a thread function. Because the calling convention automatically
        // generates a code that pops the rdi, own epilogue must be created, to prevent this action.
        asm!(
            "pop rdi",

            "cmp {0:r}, 0x0",    // If the thread must obtain some inputs, do:
            "cmovne rdi, {0:r}", // This line only changes the behavior.
            
            "add rsp, 0xa0",
            "pop rax",
            "pop rcx",
            "pop rdx",
            "pop rsi",
            "pop r8",
            "pop r9",
            "pop r10",
            "pop r11",
            "iretq",
            in(reg) thread_input,
        );
    }

    /// A helper function for calling the new thread, with fast-call calling convention
    ///
    /// This function calls the inner function of the thread and passes the thread itself
    /// as a mutable reference argument. This function must be divergent, because we are never
    /// calling it ourselves but only jumping to it's address.
    pub(crate) unsafe fn task_switch_call(t: &mut Thread) -> ! {
        use crate::kernel_components::task_virtualization::{PriorityScheduler, Task, JoinHandle};
        use core::mem;

        let closure = t.fun.as_ref();

        // Running the closure of the thread.
        let output = closure(
            mem::transmute(
                t as *const _ as usize
            )
        );

        interrupt::with_int_disabled(|| {
            // Marking the thread as done executing.
            t.thread_state = ThreadState::FINAL;

            // Writing the data to handle and removing the writer.
            if let Some(o) = &mut t.output {
                // Writing data
                o.write(output);
                // Changing the status
                o.change_state(ThreadState::FINAL);
                // Dropping the writer reference and living None on it's place.
                drop(t.output.take());
            }
        });

        interrupt::with_int_disabled(|| {
            // The PC will get here once the task is done. At this moment the task is
            // not needed anymore and can be removed.
            ROUND_ROBIN.delete(
                Task { pid: t.pid, tid: t.tid }
            );
        });

        interrupt::with_int_disabled(|| {
            // Trying to cleanup the process.
            PROCESS_MANAGEMENT_UNIT.remove(t.pid);
        });

        loop {}
    }

    /// Software keyboard interrupt handler
    ///
    /// This handler reads the data from the keyboard key that is pressed and puts it in a VGA
    /// buffer. TODO! Abstract the keyboard handling and change the output buffer for the keyboard
    /// controlls with other aplications in some queue style.
    #[no_mangle]
    unsafe extern "x86-interrupt" fn keyboard_interrupt_handler(stack_frame: InterruptStackFrame) {
        use crate::kernel_components::drivers::keyboards::GLOBAL_KEYBORD;
        use crate::kernel_components::arch_x86_64::interrupts;

        let scancode = PS2::new().read_data();
        
        interrupts::with_int_disabled(|| {
            let mut keyboard = GLOBAL_KEYBORD.lock();

            if let Ok(Some(keycode)) = keyboard.scan_key(scancode) {
                if let Some(key) = keyboard.scan_char(keycode) {
                    print!("{}", key);
                }
            }
        });
        PROGRAMMABLE_INTERRUPT_CONTROLLER.lock().master.end_of_interrupt();
    }

    /// A timer interrupt handler.
    /// 
    /// This handler will be used to switch between different threads and make the
    /// virtualization part of the OS possible. The timer interrupt is essential in
    /// performing task scheduling, time sharing, event handling and power management.
    /// 
    /// # Warn
    /// 
    /// Remap the PIC controller to work properly.
    pub const TIMER_INTERRUPT: HandlerFunction = timer_interrupt_handler;

    /// A keyboard interrupt handler.
    /// 
    /// This handler reads the value written in data port of the PS/2 controller, which will
    /// decode the received scancode and write it into the VGA buffer.
    /// 
    /// When writing your own interrupt handler for PS/2 keyboard, do not forget to read the
    /// scancode from the data por of the PS/2 controller.
    pub const KEYBOARD_INTERRUPT: HandlerFunction = keyboard_interrupt_handler;
}

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


