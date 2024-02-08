
#![no_std]
#![cfg_attr(test, no_main)]
#![allow(incomplete_features, unused, non_snake_case)]
#![feature(custom_test_frameworks, used_with_arg, error_in_core, ptr_metadata, 
    generic_const_exprs, allocator_api, slice_ptr_get, maybe_uninit_array_assume_init, 
    abi_x86_interrupt, asm_const, type_alias_impl_trait, tuple_trait, unboxed_closures)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]

/// # Project
/// Main library space. It acts as a entry point of the crate.
/// The idea behind this project is to create a OS fully from scratch without using anything but tools that compiler is offering.
/// Sometimes temporary imports may appear for some specific amount of time, but they will be replaced for own implementations that,
/// I hope, will be robust for this specific OS.
/// 
/// # PS
/// Things will get commented from time to time because this semi-stolen library can be used for own OS implementations. Different architectures will
/// be added in a far future. Some implementations may disappear in the future as the library will get optimized and change to not plagiarize
/// other's code fully from top to bottom. Some implementations may change a lot because author will gain more knowledge in this field and improve 
/// performance of the overall code. Since author is learning by creating this whole thing out, there may be a lot of dumb decisions and implementations
/// that, hopefully, will be fixed and optimized out.
/// 
/// # Knowledge
/// This is a set of knowledge oceans, both practical and theory based, that make this project possible:
/// - Writing an OS in Rust (First Edition) Philipp Oppermann's blog: https://os.phil-opp.com/edition-1/ 
/// - Writing an OS in Rust (Second edition) Philipp Oppermann's blog: https://os.phil-opp.com/
/// - OSDev wiki: https://wiki.osdev.org/Expanded_Main_Page
/// - The Art of Multiprocessor Programming by Maurice Herlihy, Nir Shavit, Victor Luchangco, Michael Spear
/// - Rustonomicon: https://doc.rust-lang.org/nomicon/
/// - Operating Systems: Three Easy Pieces by Remzi Arpaci-Dusseau, Andrea Arpaci-Dusseau
/// - MMURTL V1.0 by Richard A. Burgess
/// - Rust Cookbook https://github.com/rust-lang-nursery/rust-cookbook
/// - x86 arch source information: www.sandpile.org
/// - x86_64 arch source: https://www.intel.com/content/dam/www/public/us/en/documents/manuals/64-ia-32-architectures-software-developer-vol-3a-part-1-manual.pdf
/// 
/// # Additional Info
/// Every single outer files will be inside the kernel_components dir.
/// Every single macro can be accessed within this crate. The main components will be also accessed from here.
/// The library can be used to rewrite the main kernel, therefore there will be forks of main kernel implementation. 

/// Alloc crate for convenient allocations.
extern crate alloc;

/// Main entry point for outer structures and objects
pub mod kernel_components {
    pub mod vga_buffer;

    pub mod structures {
        pub mod single;
        pub mod iternum;
        pub mod bytes;
        pub mod bitflags;

        pub mod thread_safe {
            pub mod concurrent_list;
            pub mod concurrent_queue;

            pub use concurrent_list::ConcurrentList;
            pub use concurrent_queue::ConcurrentQueue;
        }

        pub use bytes::{AsBytes, Bytes};
        pub use iternum::IternumTrait;
        pub use single::{Once, Single};
        pub use bitflags::BitNode;
    }

    pub mod arch_x86_64 {
        pub mod privilege_rings;
        pub mod descriptor_table;
        pub mod random;
        pub mod post;
        pub mod ports;
        pub mod TLB;

        pub mod interrupts {
            pub mod handler_functions;
            pub mod interrupt;
            pub mod interrupt_descriptor_table;

            pub use handler_functions::HandlerFn;
            pub use interrupt_descriptor_table::{GateDescriptor, IDT, GateType, INTERRUPT_DESCRIPTOR_TABLE};
            pub use interrupt::{
                cause_interrupt, cause_interrupt_unsafe,
                enable, disable, with_int_disabled, with_int_enabled,
                wait_for_interrupt,
                breakpoint, 
                divide_by_zero, 
                hlt
            };
        }

        pub mod controllers {
            pub mod ps_2;
            pub mod pic;
            pub mod apic;

            pub mod pic_command_words;

            pub use pic::{PIC, PROGRAMMABLE_INTERRUPT_CONTROLLER};
            pub use ps_2::{PS2, PSControllerCommand, PSControllerConfiguration};
        }

        pub mod segmentation {
            pub mod global_descriptor_table;
            pub mod task_state_segment;

            pub use task_state_segment::TSS;
            pub use global_descriptor_table::{SegmentDescriptor, SegmentSelector, GDT, GLOBAL_DESCRIPTOR_TABLE};
        }

        pub use descriptor_table::DTPointer;
        pub use privilege_rings::PrivilegeLevel;
        pub use random::{RdRand, RdSeed};
    }

    pub mod registers {
        pub mod segment_regs;
        pub mod control;
        pub mod mxscr;
        pub mod flags;
        pub mod ms;
    }

    pub mod drivers;

    pub mod sync {
        pub mod mutex;

        pub use mutex::{Mutex, MutexGuard};
    }

    pub mod memory {
        
        pub mod allocators {
            pub mod global_alloc;
            pub mod leak_alloc;
            pub mod bump_alloc;
            pub mod node_alloc;
            pub mod free_list_alloc;

            pub use global_alloc::{GAllocator, SubAllocator, GLOBAL_ALLOCATOR};
            pub use leak_alloc::{LeakAlloc, LEAK_ALLOC};
            pub use bump_alloc::{BumpAlloc, BUMP_ALLOC};
            pub use node_alloc::{NodeAlloc, NODE_ALLOC};
            pub use free_list_alloc::{FreeListAlloc, FREE_LIST_ALLOC};
        }

        pub mod stack_allocator;

        pub mod memory_module;
        pub mod memory_map;
        pub mod sections;
        pub mod tags;

        pub mod frames;
        pub mod paging;
        pub mod owned_tables;
        pub mod temporary_pages;
        pub mod inactive_tables;

        pub use memory_module::{MMU, InfoPointer, BootInfoHeader, MEMORY_MANAGEMENT_UNIT};
        pub use frames::AreaFrameAllocator;
        pub use stack_allocator::StackAlloc;
        
        pub use paging::{Page, Table, Entry, EntryFlags};
        pub use owned_tables::ActivePageTable;
        pub use temporary_pages::TempPage;
        pub use inactive_tables::InactivePageTable;
    }

    pub mod task_virtualization {
        pub mod scheduler;
        pub mod round_robin;
        pub mod priority_based_scheduling;
        
        pub mod process;
        pub mod thread;
        pub mod pmu;

        pub use pmu::{PMU, PROCESS_MANAGEMENT_UNIT};
        pub use process::{Process, ProcState};
        pub use thread::{Thread, ThreadFn, ThreadOutput, ThreadState};
        pub use scheduler::{Scheduler, Task};

        pub use round_robin::{ROUND_ROBIN, RoundRobin};
        pub use priority_based_scheduling::{PRIORITY_SCHEDULER, PriorityScheduler};
    }

}

/// Custom types for overall use and better readability.
pub type PhysicalAddress = usize;
pub type VirtualAddress = usize;

use core::panic::PanicInfo;

/// Those are some short-cuts for some features that often used.
pub use kernel_components::{
    
    structures::{
        bytes::{Bytes, AsBytes},
        thread_safe,
    },

    memory::{
        allocators::{GLOBAL_ALLOCATOR, LEAK_ALLOC, BUMP_ALLOC, NODE_ALLOC, FREE_LIST_ALLOC},
    },

    vga_buffer::Color,
};

/// This function will be called on fatal errors in the system.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    #[cfg(test)]
    println!(Color::RED; "[failed]");
    println!(Color::RED; "{}", info);
    
    loop {}
}

/// Custom trait for tests. It is only used when testing and do not affect overall performance.
pub trait Testable {
    fn run(&self) -> ();
}

impl<T> Testable for T where T: Fn(), {
    fn run(&self) {
        print!("{}...    ", core::any::type_name::<T>());
        self();
        println!(Color::GREEN; "[ok]");
    }
}

/// Test runner for testing kernel components. It wil run all unit tests as well as integrated one.
/// TODO! Fix the way how printing works, to make it readable.
pub fn test_runner(tests: &[&dyn Fn()]) -> ! {
    println!(Color::LIGHTBLUE; "Running {} tests:", tests.len());
    for test in tests {
        test.run();
    }

    loop {}
}
