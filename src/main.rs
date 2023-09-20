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
use notOS::{warn, kernel_components::{memory::{self, memory_module::{InfoPointer, BootInfoHeader}, allocators::SubAllocator}, registers::control}, Color, GLOBAL_ALLOCATOR, NODE_ALLOC};

#[no_mangle]
pub extern "C" fn _start(_multiboot_information_address: usize) {
    // All tests will be trapped here instantly. This must be this way,
    // because memory manipulations may cause undefined behavior for tests.
    #[cfg(test)]
    test_main();
    
    // Memory initialization.
    {
        let boot_info = unsafe { InfoPointer::load(_multiboot_information_address as *const BootInfoHeader ) }.unwrap();
        
        // The global allocator is a mutable static that do not use any locking 
        // algorithm, so any operation on it, is unsafe.
        unsafe { GLOBAL_ALLOCATOR.r#use(&NODE_ALLOC) };
        
        control::Cr0::enable_write_protect_bit();
        memory::init(&boot_info);
    }

    // This part will only be compiled during debugging.
    #[cfg(debug_assertions)] {
        warn!("DEBUG MODE ON!");
    }
    
    main();
}

#[allow(dead_code, unreachable_code)]
fn main() -> ! {
    use notOS::Vec;
    {
        let mut vector: Vec<u8> = Vec::new();
    
        for i in 0..30 {
            vector.push(i);
        }
    }

    loop {}
}