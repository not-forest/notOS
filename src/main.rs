#![no_std]
#![no_main]
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
        memory::{InfoPointer, BootInfoHeader},              
    Color,
};

#[no_mangle]
pub extern "C" fn _start(_multiboot_information_address: usize) {
    #[cfg(debug_assertions)] {
        let boot_info = unsafe { InfoPointer::load(_multiboot_information_address as *const BootInfoHeader ) }.unwrap();
        let memory_map_tag = boot_info.memory_map_tag()
            .expect("Memory map tag required.");

        println!("Memory Areas:");
        for area in memory_map_tag.memory_areas() {
            println!(Color::LIGHTGREEN; "      start: 0x{:x}, length: 0x{:x}", area.base_addr, area.length);
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