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
        registers::{control, ms}}, 
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
        GateDescriptor,
    };

    use notOS::kernel_components::arch_x86_64::controllers::PROGRAMMABLE_INTERRUPT_CONTROLLER;

    // Memory initialization.
    // The global allocator is a mutable static that do not use any locking 
    // algorithm, so any operation on it, is unsafe.
    unsafe { 
        GLOBAL_ALLOCATOR.r#use(&FREE_LIST_ALLOC);
    
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
        INTERRUPT_DESCRIPTOR_TABLE.push(0, gate_div);
        INTERRUPT_DESCRIPTOR_TABLE.push(3, gate_break);
        INTERRUPT_DESCRIPTOR_TABLE.push(8, gate_double_fault);
        INTERRUPT_DESCRIPTOR_TABLE.push(14, gate_page_fault);

        INTERRUPT_DESCRIPTOR_TABLE.push(32, gate_timer);
        INTERRUPT_DESCRIPTOR_TABLE.push(33, gate_keyboard);

        // Loading the IDT table to the CPU.
        INTERRUPT_DESCRIPTOR_TABLE.load_table();

        // Remapping the PIC controller.
        PROGRAMMABLE_INTERRUPT_CONTROLLER.lock().reinit_chained(32).remap();

        interrupts::enable();
    
        use notOS::kernel_components::task_virtualization::{Process, PROCESS_MANAGEMENT_UNIT};
        let stack = MEMORY_MANAGEMENT_UNIT.allocate_stack(6).unwrap();
        
        let mut p1 = Process::new(
            stack, 
            1024, 
            1,
            None,
            || {   
                println!("Hello. Main");

                loop {}
            },
        );

        p1.spawn(|| {
            println!("Hello. Thread.");

            loop {}
        });

        PROCESS_MANAGEMENT_UNIT.queue(p1); 
    }

    main()
}

#[allow(dead_code, unreachable_code)]
fn main() -> ! {
    println!("Tasking soon!");

    loop {}
}