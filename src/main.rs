#![no_std]
#![no_main]
#![allow(non_snake_case)]
#![feature(custom_test_frameworks, used_with_arg, abi_x86_interrupt, asm_const)]
#![test_runner(notOS::test_runner)]
#![reexport_test_harness_main = "test_main"]

#[link(name = "bootloader")]
extern "C" {
    fn initiate();
    fn header_start();
    fn header_end();
}

// The alloc crate will be useful for the main kernel binary no matter what.
extern crate alloc;

#[used]
static INITIATE_FUNC: unsafe extern "C" fn() = initiate;
#[used(linker)]
static HEADER_START_FUNC: unsafe extern "C" fn() = header_start;
#[used(linker)]
static HEADER_END_FUNC: unsafe extern "C" fn() = header_end;

/// This is the main binary (kernel) space. As the library will build in, more new features will be added further.
use notOS::{
    println, warn, single,
    kernel_components::{
        arch_x86_64::interrupts,
        memory::MEMORY_MANAGEMENT_UNIT, 
        registers::{control, ms},
    }, 
        GLOBAL_ALLOCATOR, FREE_LIST_ALLOC,
    };

#[no_mangle]
pub extern "C" fn _start(_multiboot_information_address: usize) {
    // All tests will be trapped here instantly. This must be this way,
    // because memory manipulations may cause undefined behavior for tests.
    #[cfg(test)]
    test_main();

    // This part will only be compiled during debugging.
    #[cfg(debug_assertions)] {
        warn!("DEBUG MODE ON!");
    }

    use notOS::kernel_components::arch_x86_64::PrivilegeLevel;

    use notOS::kernel_components::registers::segment_regs::{
        Segment, SegmentSelector, StackSegment, CodeSegment
    };

    use notOS::kernel_components::arch_x86_64::segmentation::{
        TSS, GDT, GLOBAL_DESCRIPTOR_TABLE
    };

    use notOS::kernel_components::arch_x86_64::interrupts::{
        handler_functions::predefined::*,
        handler_functions::software::*,
        INTERRUPT_DESCRIPTOR_TABLE,
        InterruptVector, 
        GateDescriptor,
    };

    use notOS::kernel_components::arch_x86_64::controllers::PROGRAMMABLE_INTERRUPT_CONTROLLER;

    // Memory initialization.
    // The global allocator is a mutable static that do not use any locking 
    // algorithm, so any operation on it, is unsafe.
    unsafe { 
        GLOBAL_ALLOCATOR.r#use(&FREE_LIST_ALLOC);
        FREE_LIST_ALLOC.change_strategy(
            notOS::kernel_components::memory::allocators::free_list_alloc::SearchStrategy::BEST_FIT
        );
   
        // New MMU structure makes it easier to handle memory related commands.
        MEMORY_MANAGEMENT_UNIT.init(_multiboot_information_address);
    };
    
    // Enabling the nxe bit and write protect bit.
    control::Cr0::enable_write_protect_bit();
    ms::EFER::enable_nxe_bit();

    single! {
        mut TASK_STATE_SEGMENT: TSS = TSS::new();
    }

    unsafe {
        // Setting up the stack for IST.
        MEMORY_MANAGEMENT_UNIT.set_interrupt_stack(&mut TASK_STATE_SEGMENT,0,1);

        // Rewrite the static GDT. It will use the flat setup.
        GLOBAL_DESCRIPTOR_TABLE.reinit(GDT::flat_setup(&TASK_STATE_SEGMENT));
        GLOBAL_DESCRIPTOR_TABLE.load_table(); // Loads the table to the CPU.

        // Reloading the CS segment.
        CodeSegment::write(
            SegmentSelector::new(1, false, PrivilegeLevel::KernelLevel)
        );

        // Reloading the SS segment.
        StackSegment::write(
            SegmentSelector::new(2, false, PrivilegeLevel::KernelLevel)
        );

        // Reloading the TSS segment.
        TSS::write(
            SegmentSelector::new(5, false, PrivilegeLevel::KernelLevel)
        );

        // Exception gates.
        let gate_div = GateDescriptor::new_trap(DIVISION_BY_ZERO);
        let gate_break = GateDescriptor::new_trap(BREAKPOINT);
        let gate_double_fault = GateDescriptor::new_trap(DOUBLE_FAULT);
        let gate_page_fault = GateDescriptor::new_trap(PAGE_FAULT);

        // Interrupt gates.
        let gate_timer = GateDescriptor::new_interrupt(TIMER_INTERRUPT);

        let gate_keyboard = GateDescriptor::new_interrupt(KEYBOARD_INTERRUPT);

        // Pushing the gates into the IDT.
        INTERRUPT_DESCRIPTOR_TABLE.push(InterruptVector::DIVIDE_BY_ZERO, gate_div);
        INTERRUPT_DESCRIPTOR_TABLE.push(InterruptVector::BREAKPOINT, gate_break);
        INTERRUPT_DESCRIPTOR_TABLE.push(InterruptVector::DOUBLE_FAULT, gate_double_fault);
        INTERRUPT_DESCRIPTOR_TABLE.push(InterruptVector::PAGE_FAULT, gate_page_fault);

        INTERRUPT_DESCRIPTOR_TABLE.push(
            InterruptVector::PICMappings(32), gate_timer
        );
        INTERRUPT_DESCRIPTOR_TABLE.push(
            InterruptVector::PICMappings(33), gate_keyboard
        );

        // Loading the IDT table to the CPU.
        INTERRUPT_DESCRIPTOR_TABLE.load_table();

        // Remapping the PIC controller.
        PROGRAMMABLE_INTERRUPT_CONTROLLER.lock().reinit_chained(32).remap();
    
        use notOS::kernel_components::task_virtualization::{Process, PROCESS_MANAGEMENT_UNIT};
        let stack1 = MEMORY_MANAGEMENT_UNIT.allocate_stack(10).unwrap();

        let p1 = Process::new_void(stack1, 0, 1, 1, None,
            |t| {
                use notOS::Color;
                use notOS::kernel_components::sync::Mutex;
                use alloc::sync::Arc;      

                println!(Color::MAGENTA; "Hello from the main thread.");
                let data = Arc::new(Mutex::new(0));
             
                for _ in 0..10 {
                    let data = Arc::clone(&data);

                    let handle = t.spawn(move |_| {
                        let mut value = data.lock();
                        *value += 1;
                        println!(Color::GREEN; "Current value is: {}", *value);
                    });

                    handle.join().ok();
                }

                println!(Color::MAGENTA; "Result value is: {}", *data.lock());
            },
        );

        // Pushing the process to the queue.
        PROCESS_MANAGEMENT_UNIT.queue(p1);
    }

    loop {
        // Waiting for interrupts to happen.
        interrupts::wait_for_interrupt();
    }
}
