/// Defines predefined interrupts handle functions for use in the OS. 
///
/// For any of this function, the interrupt controller must be reprogrammed to the desired
/// interrupt vector.

use alloc::boxed::Box;
use core::any::Any;
use core::arch::asm;

use crate::kernel_components::arch_x86_64::interrupts::{interrupt, INTERRUPT_DESCRIPTOR_TABLE};
use crate::kernel_components::drivers::{
    DRIVER_MANAGER, DriverType,
    keyboards::KeyboardDriver,
    interrupts::InterruptControllerDriver,
};
use crate::kernel_components::keyboard_interface::OS_CHAR_BUFFER;
use crate::kernel_components::memory::EntryFlags;
use crate::kernel_components::task_virtualization::{Thread, Scheduler, PROCESS_MANAGEMENT_UNIT, PRIORITY_SCHEDULER, ROUND_ROBIN, ThreadState};
use crate::{critical_section, debug, handler_function_prologue, print, println, warn, Color};
use super::handler_functions::*;

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
    use core::sync::atomic::Ordering;

    handler_function_prologue!(32);

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
    critical_section!(|| {
        // Getting the lock.
        let mut pmu = PROCESS_MANAGEMENT_UNIT.process_list.lock();

        // Trying to obtain current task, if it exists.
        if let Some(task) = ROUND_ROBIN.current() { 
            if let Some(process) = pmu.get_mut(task.pid) {
                if let Some(thread) = process.find_thread_mut(task.tid) {
                    let save = || {
                        /* debug!("SAVING TO THREAD NR: {} with {:?}, {:x}, {:x}", 
                            task.tid, thread.thread_state, stack_frame.instruction_pointer, stack_frame.stack_ptr); */
                        // Writting the old values to the thread.
                        let _ = thread.stack_ptr.fetch_update(
                            Ordering::SeqCst, Ordering::SeqCst, |_| Some(stack_frame.stack_ptr)
                        );
                        let _ = thread.instruction_ptr.fetch_update(
                            Ordering::SeqCst, Ordering::SeqCst, |_| Some(stack_frame.instruction_pointer)
                        );
                    };

                    // Doing some tasks to the previous thread, based on it's state.
                    match thread.thread_state {
                        // All running-like codes, which suppose to save their work.
                        ThreadState::RUNNING | ThreadState::PREFINALIGNORE => save(),
                        ThreadState::PREFINAL => { save(); thread._final() },
                        ThreadState::PREHALT(isr) => { save(); thread._halt(isr) },
                        _ => (), // The rest will be ignored.
                    }
                }
            }
        }
        // Trying to obtain some new tasks from a scheduler if some.
        loop {
            if let Some(task) = ROUND_ROBIN.schedule() {
                // Trying to find the process by task's pid.
                //
                // If not exists, we can easily delete all tasks with this pid.
                if let Some(process) = pmu.get_mut(task.pid) {
                    // Trying to find the thread by task's tid.
                    //
                    // If not exists, we can delete this specific task.
                    if let Some(thread) = process.find_thread_mut(task.tid) {
                        // Doing different tasks based on the thread state.
                        match thread.thread_state {
                            ThreadState::INIT => {
                                // If the thread is only about to run we must provide some additional information for it.
                                //
                                // This basically means that we must perform the fast-call calling convention manually, so the
                                // thread can use a mutable reference to itself and perform the instructions.
                                thread_input = thread as *const _ as usize;

                                stack_frame.instruction_pointer = // Entering the task switch function.
                                    crate::kernel_components::task_virtualization::thread::task_switch_call as usize;
                                stack_frame.stack_ptr = thread.stack_ptr.load(Ordering::Acquire);
                                
                                /* debug!("PUSH TO THREAD NR: {} with {:?}, {:x}, {:x}", 
                                    task.tid, thread.thread_state, stack_frame.instruction_pointer, stack_frame.stack_ptr); */

                                // Changing the state to running, which will not affect the thread's input.
                                thread._running();
                                break;
                            },
                            ThreadState::FINAL => {
                                // Killing the task.
                                ROUND_ROBIN.delete(*task);
                                break;
                            },
                            ThreadState::HALT(isr) => {
                                // If isr is set within the IDT - it is time to unhalt.
                                if INTERRUPT_DESCRIPTOR_TABLE
                                    .with_int(isr, |bit| {let tmp = *bit; *bit = false; tmp == true}) 
                                {
                                    thread._running();
                                } else { continue }
                            },
                            _ => (),
                        }

                        // Changing the current stack pointer to the thread's ones.
                        stack_frame.stack_ptr = thread.stack_ptr.load(Ordering::Acquire);
                        stack_frame.instruction_pointer = thread.instruction_ptr.load(Ordering::Acquire);
                        /* debug!("PUSH TO THREAD NR: {} with {:?}, {:x}, {:x}", 
                            task.tid, thread.thread_state, stack_frame.instruction_pointer, stack_frame.stack_ptr); */
                        break;
                    } else {
                        // If there are no underlying threads we must delete the hangling task
                        ROUND_ROBIN.delete(*task);
                    }
                } else {
                    // If there are no underlying process, we must delete the hangling task
                    ROUND_ROBIN.delete(*task);
                }
            }    
        }
    });

    DRIVER_MANAGER
        .driver::<Box<dyn InterruptControllerDriver>>(DriverType::Interrupt)
        .map_or(
            crate::error!("Unable to handle interrupt prologue. No interrupt controller driver available."), 
            |d| d.prologue(stack_frame)
        );

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

/// Software keyboard interrupt handler
///
/// This handler reads the data from the keyboard key that is pressed and puts it in a VGA
/// buffer. TODO! Abstract the keyboard handling and change the output buffer for the keyboard
/// controlls with other aplications in some queue style.
#[no_mangle]
unsafe extern "x86-interrupt" fn keyboard_interrupt_handler(stack_frame: InterruptStackFrame) {
    handler_function_prologue!(33);

    if let Some(intd) = DRIVER_MANAGER
        .driver::<Box<dyn InterruptControllerDriver>>(DriverType::Interrupt) {

        critical_section!(|| {

            if let Some(keyboard) = DRIVER_MANAGER.driver::<Box<dyn KeyboardDriver>>(DriverType::Keyboard) {
                // If key exist, writing data to the buffer so that applications can use it.
                if let Some(key) = keyboard.read() {
                    OS_CHAR_BUFFER.lock().append(key)
                }
            } else {
                use crate::kernel_components::arch_x86_64::controllers::PS2;
                warn!("Keyboard input detected, yet ignored due to no available keyboard driver found.");
                // This allows PIC to send more keyboard interrupts.
                let _ = PS2::new().read_data();
            }
        });

        intd.prologue(stack_frame);
    } else {
        crate::error!("Unable to handle interrupt prologue. No interrupt controller driver available."); 
    }
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

