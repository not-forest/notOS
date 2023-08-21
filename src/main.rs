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
    kernel_components::
        memory::{InfoPointer, BootInfoHeader, AreaFrameAllocator, frames::FrameAlloc},            
    Color,
};

#[no_mangle]
pub extern "C" fn _start(_multiboot_information_address: usize) {
    #[cfg(debug_assertions)] {
        let boot_info = unsafe { InfoPointer::load(_multiboot_information_address as *const BootInfoHeader ) }.unwrap();
        let memory_map_tag = boot_info.memory_map_tag()
            .expect("Memory map tag required.");
        let elf_sections_tag = boot_info.elf_sections_tag()
            .expect("Elf-sections tag required.");

        let kernel_start = boot_info.kstart();
        let kernel_end = boot_info.kend();
        let multiboot_start = _multiboot_information_address;
        let multiboot_end = multiboot_start + ( boot_info.total() as usize );

        /*
        println!("Memory Areas:");
        for area in memory_map_tag.memory_areas() {
            println!(Color::GREEN; "      start: 0x{:x}, length: 0x{:x}", area.base_addr, area.length);
        }

        println!("Kernel Sections:");
        for (num, section) in elf_sections_tag.enumerate() {
            let section_inner = section.get();
            println!(Color::LIGHTGREEN; "      addr: 0x{:x}, size: 0x{:x}, flags: 0x{:x}, number: {}", section_inner.addr(), section_inner.size(), section_inner.flags(), num);
        }
        */

        let mut frame_allocator = AreaFrameAllocator::new(
            kernel_start as usize, 
            kernel_end as usize, 
            multiboot_start, 
            multiboot_end,
            memory_map_tag.memory_map_iter(),
        );

        for i in 0.. {
            if let None = frame_allocator.alloc() {
                println!(Color::MAGENTA; "Allocated {} frames", i);
                break;
            }
        }
    }    
    

    #[cfg(test)]
    test_main();

    main();
}

#[allow(dead_code, unreachable_code)]
fn main() -> ! {
    println!(Color::BLUE; "Hello memory!");
    
    loop {}
}