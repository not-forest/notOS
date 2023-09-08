#![no_std]
#![no_main]
#![allow(non_snake_case)]
#![feature(custom_test_frameworks, used_with_arg)]
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
use notOS::{println, print, 
    kernel_components::{
        memory::{
            InfoPointer, BootInfoHeader, 
            AreaFrameAllocator,
            self,
        },
        registers::control,
    },            
    Color,
};

#[no_mangle]
pub extern "C" fn _start(_multiboot_information_address: usize) {
    // All tests will be trapped here instantly. This must be this way,
    // because memory manipulations may cause undefined behavior for tests.
    #[cfg(test)]
    test_main();
    
    // This part will only be compiled during debugging.
    #[cfg(debug_assertions)] {
        let boot_info = unsafe { InfoPointer::load(_multiboot_information_address as *const BootInfoHeader ) }.unwrap();
        let memory_map_tag = boot_info.memory_map_tag()
            .expect("Memory map tag required.");
        let elf_sections_tag = boot_info.elf_sections_tag()
            .expect("Elf-sections tag required.");

        let kernel_start = boot_info.kstart();
        let kernel_end = boot_info.kend();
        let multiboot_start = boot_info.mstart();
        let multiboot_end = boot_info.mend();

        let mut frame_allocator = AreaFrameAllocator::new(
            kernel_start as usize, 
            kernel_end as usize, 
            multiboot_start, 
            multiboot_end,
            memory_map_tag.memory_map_iter(),
        );

        control::Cr0::enable_write_protect_bit();

        println!("Time to remap!");
        // remaping the kernel
        memory::remap_kernel(&mut frame_allocator, &boot_info);
        println!("Kernel sections are now remapped!");
    }   

    main();
}

#[allow(dead_code, unreachable_code)]
fn main() -> ! {
    println!(Color::BLUE; "Hello memory!");

    loop {}
}