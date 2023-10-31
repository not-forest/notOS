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
        memory::MMU, 
        registers::{control, ms}}, 
        Color, GLOBAL_ALLOCATOR, BUMP_ALLOC,
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
        INTERRUPT_DESCRIPTOR_TABLE,
        GateDescriptor,
    };

    // Memory initialization.
    // The global allocator is a mutable static that do not use any locking 
    // algorithm, so any operation on it, is unsafe.
    unsafe { GLOBAL_ALLOCATOR.r#use(&BUMP_ALLOC) };
    
    // New MMU structure makes it easier to handle memory related commands.
    let mut MEMORY_MANAGEMENT_UNIT = MMU::new_init(_multiboot_information_address);

    control::Cr0::enable_write_protect_bit();
    ms::EFER::enable_nxe_bit();

    single! {
        mut TASK_STATE_SEGMENT: TSS = TSS::new();
    }

    unsafe {
        MEMORY_MANAGEMENT_UNIT.set_interrupt_stack(&mut TASK_STATE_SEGMENT,0,1);

        GLOBAL_DESCRIPTOR_TABLE.reinit(GDT::flat_setup(&TASK_STATE_SEGMENT));
        GLOBAL_DESCRIPTOR_TABLE.load_table();

        CodeSegment::write(
            SegmentSelector::new(1, false, PrivilegeLevel::KernelLevel)
        );

        StackSegment::write(
            SegmentSelector::new(2, false, PrivilegeLevel::KernelLevel)
        );

        TSS::write(
            SegmentSelector::new(5, false, PrivilegeLevel::KernelLevel)
        );

        let gate_div = GateDescriptor::new_trap(DIVISION_BY_ZERO);
        let gate_break = GateDescriptor::new_trap(BREAKPOINT);
        let gate_double_fault = GateDescriptor::new_trap(DOUBLE_FAULT);
        let gate_page_fault = GateDescriptor::new_trap(PAGE_FAULT);

        INTERRUPT_DESCRIPTOR_TABLE.push(0, gate_div);
        INTERRUPT_DESCRIPTOR_TABLE.push(3, gate_break);
        INTERRUPT_DESCRIPTOR_TABLE.push(8, gate_double_fault);
        INTERRUPT_DESCRIPTOR_TABLE.push(14, gate_page_fault);
        INTERRUPT_DESCRIPTOR_TABLE.load_table();
    }
    
    main();
}

#[allow(dead_code, unreachable_code)]
fn main() -> ! {

    loop {}
}