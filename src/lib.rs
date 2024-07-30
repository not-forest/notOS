#![no_std]
#![cfg_attr(test, no_main)]
#![allow(incomplete_features, unused, non_snake_case, static_mut_refs)]
#![feature(custom_test_frameworks, used_with_arg, error_in_core, ptr_metadata, slice_from_ptr_range,
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

/// Main library entry point.
pub mod kernel_components {
    /// I/O operation on VGA buffer (Basic TUI)
    pub mod vga_buffer;
    /// OS specific helper types.
    pub mod os;

    /// Custom data structures and types for operating on OS resources.
    ///
    /// This module consists of important structures, which are required for proper OS work.
    /// Different parts of other code might use those structures for example for critical code
    /// sections that require thread safe wrappers.
    pub mod structures {
        /// Defines structures for constant one-time initialization per OS load.
        pub mod single;
        /// Defines a helper trait that works with outer proc_macro trait. Allows to iterate
        /// through enums to reduce code's size.
        pub mod iternum;
        /// Custom trait to represent any value as a reference to a pack of bytes.
        pub mod bytes;
        /// Custom data structures to represent numerical values as a bitfield for convenient
        /// operations on bits.
        pub mod bitflags;

        /// Thread safe Data Structures.
        pub mod thread_safe {
            /// Lock-free concurrent list data structure. Ensures mutual exclution by applying
            /// atomic flags and pointers to each individual node and manipulating them to perform
            /// atomic operations on nodes themselves.  
            pub mod concurrent_list;
            /// Lock-free queue data structure based on Michael & Scott algorithm.
            pub mod concurrent_queue;

            pub use concurrent_list::ConcurrentList;
            pub use concurrent_queue::ConcurrentQueue;
        }

        pub use bytes::{AsBytes, Bytes};
        pub use iternum::IternumTrait;
        pub use single::{Once, Single};
        pub use bitflags::BitNode;
    }

    /// Main module that defines x86 specific structures and interfaces. The majority of those
    /// modules are compatible with 32-bit version, however meant to be used for 64-bit version.
    pub mod arch_x86_64 {
        /// Protection rings implementation in form of enum.
        pub mod privilege_rings;
        /// Defines interface to read and write descriptor tables.
        pub mod descriptor_table;
        /// Defines structures to obtain hardware generated random values.
        pub mod random;
        /// Defines a port for POST debug boards. (Micro-delay abuse)  
        pub mod post;
        /// Defines trait that represents CPU ports of different sizes.
        pub mod ports;
        /// Manipulations with the Transition Lookaside Buffer.
        pub mod TLB;

        /// This module defines all ACPI related structures and procedures.
        ///
        /// It contains the most used ACPI tables to manipulate with power settings and perform
        /// ACPI-related tasks. Those tables can be exposed and used manually by the OS, or via
        /// acpi_service modules, that defines several wrapper functions.
        pub mod acpi {
            /// Main ACPI module that defines common structures used in all ACPI tables and a
            /// service for convenient interface with OS.
            pub mod acpi;
            /// Defines RSDP/XSDP pointer table. The main entry of ACPI journey.
            pub mod rsdp;
            /// Defines RSDT/XSDT tables. Those tables contain a list of pointers to ACPI fixed
            /// tables.
            pub mod rsdt;
            /// Main ACPI tables that defines hardware features and allows to manipulate with it
            /// via it's mapped registers. 
            pub mod fadt;

            /// This module defines differentiated ACPI tables and AML language interpreter.
            pub mod diff {
                /// Module that packs all parser related sub-modules
                mod parser {
                    /// Parsing package length encoding.
                    mod pkg;
                    /// Main parser structure. Used as an interface to other modules by interpreter
                    mod aml_parser;
                    /// Defines AML definitions, like Names, Scopes, Aliases, etc.
                    mod definitions;

                    pub use aml_parser::{AMLParser, AMLParserError, AMLParserResult};
                    pub use pkg::PkgLength;
                    pub use definitions::Scope;
                }

                /// Defines different AML language data types and objects.
                mod objects;
                /// Defines an output tree-like data structure for ACPI management.
                mod namespace;
                /// Main interface to communicate between ACPI and OS.
                mod interpreter;
                /// Defines common AML data types and constants.
                pub mod aml;

                /// Defines a DSDT table that allows to build ACPI Namespace.
                pub mod dsdt;

                pub use aml::{AMLStream, AMLResult};
                pub use dsdt::DSDT;
                pub use interpreter::{AMLInterpreter, AMLInterpreterError};
                pub use parser::{AMLParser, AMLParserError};
            }

            pub use acpi::{acpi_service, XSDT, RSDT, FADT};
        }

        /// Iterrupts and exceptions handling.
        ///
        /// Defines basic interface to find, modify and load the Interrupt Descriptor Table to
        /// perform exceptions/interrupt handling. Custom written functions can be used for such
        /// purposes, however a set of pre-made functions are implemented there also.
        pub mod interrupts {
            /// This module defines HandlerFn trait that is used for filling IDT with own interrupt 
            /// handlers. Pre-defined functions are also implemented in it's submodules. 
            pub mod handler_functions;
            /// Defines common interrupt/exception structures and several wrappers for assembly
            /// instructions, which are somehow related to interrupts/exceptions. 
            pub mod interrupt;
            /// Defines an Interrupt Descriptor Table structure and it's methods.
            pub mod interrupt_descriptor_table;

            pub use handler_functions::HandlerFn;
            pub use interrupt_descriptor_table::{GateDescriptor, IDT, GateType, INTERRUPT_DESCRIPTOR_TABLE};
            pub use interrupt::{
                InterruptVector,
                cause_interrupt, cause_interrupt_unsafe,
                enable, disable, with_int_disabled, with_int_enabled,
                wait_for_interrupt,
                breakpoint, 
                divide_by_zero, 
                hlt
            };
        }

        /// Defines interfaces for different CPU inner controllers including those, that might
        /// cause IRQ interrupts. 
        pub mod controllers {
            /// PS/2 controller management (Keyboard controller for old keyboards.)
            pub mod ps_2;
            /// Programmable Interrupt Controller management. (Legacy controller.)
            pub mod pic;
            /// Advanced Programmable Interrupt Controller management.
            pub mod apic;
            /// Defines command words for PIC controllers for easy management.
            pub mod pic_command_words;

            pub use pic::{PIC, PROGRAMMABLE_INTERRUPT_CONTROLLER};
            pub use ps_2::{PS2, PSControllerCommand, PSControllerConfiguration};
        }

        /// Defines a minimal segmentation interface to jump into 64-bit Long Mode.
        pub mod segmentation {
            /// Global Descriptor Table implementation. Contains OS's segments
            pub mod global_descriptor_table;
            /// Minimal Task State Segment implementation for Long Mode.
            pub mod task_state_segment;

            pub use task_state_segment::TSS;
            pub use global_descriptor_table::{SegmentDescriptor, SegmentSelector, GDT, GLOBAL_DESCRIPTOR_TABLE};
        }

        pub use descriptor_table::DTPointer;
        pub use privilege_rings::PrivilegeLevel;
        pub use random::{RdRand, RdSeed};
    }

    /// x86 architecture-specific registers and their management. 
    pub mod registers {
        /// Defines all 6 CPU's segment registers and interface to manipulate them.
        pub mod segment_regs;
        /// Defines all CPU's control registers and methods to load/store their values 
        pub mod control;
        /// Defines MXSRC register. Associated with floating-point control and status for SIMD
        /// instructions.
        pub mod mxscr;
        /// Defines CPU status register XFLAGS. 
        pub mod flags;
        /// Defines Model Specific registers. 
        pub mod ms;
    }

    /// Custom module for driver interface.
    ///
    /// Such interfaces define code that must run in kernel-space. Basically a bridge between
    /// user-space and kernel-space to create kernel modules for system's peripherals.
    pub mod drivers;

    /// Synchronization Primitives.
    pub mod sync {
        /// Implementation of basic Mutex. Yields in multithreaded environment.
        pub mod mutex;
        /// Simple Semaphore implementation with counter.
        pub mod semaphore;
        /// Thread barrier for OS. Only works for threads within one Process.
        pub mod barrier;

        pub use mutex::{Mutex, MutexGuard};
        pub use semaphore::{Semaphore};
        pub use barrier::Barrier;
    }

    /// Module for all memory related manipulations.
    pub mod memory {
        /// Different heap memory allocators implementations.
        pub mod allocators {
            /// Module that defines a Global Allocator constant, that is being used when heap-related 
            /// tasks are called. Different allocators can be used within the global one. (Using
            /// leak allocator by default.)
            pub mod global_alloc;
            /// Most basic heap allocator implementation. Never deallocates data, always leaks memory.
            /// (Not very useful.)
            pub mod leak_alloc;
            /// Bump allocator implementation. May deallocate the previous allocation if it is the
            /// same size as a new one by using 'BumpHoles'. (Not very useful)
            pub mod bump_alloc;
            /// Fast allocator for small heaps. Uses an array of nodes to allocate values.
            /// (Arguably useful.)
            pub mod node_alloc;
            /// Free List allocator implementation with different searching techniques. Stores all
            /// data in a dynamic list structure within the heap region. (Solid choice)
            pub mod free_list_alloc;
            /// Buddy Allocator implementation. (Very solid choice ^-^)
            pub mod buddy_alloc;

            pub use global_alloc::{GAllocator, SubAllocator, GLOBAL_ALLOCATOR};
            pub use leak_alloc::{LeakAlloc, LEAK_ALLOC};
            pub use bump_alloc::{BumpAlloc, BUMP_ALLOC};
            pub use node_alloc::{NodeAlloc, NODE_ALLOC};
            pub use free_list_alloc::{FreeListAlloc, FREE_LIST_ALLOC};
            pub use buddy_alloc::{BuddyAlloc, BUDDY_ALLOC};
        }

        /// Simple allocator for stack management in Long Mode environment.
        pub mod stack_allocator;

        /// Main memory related module. Holds the MMU structure, which is a key to memory
        /// management operations and initialization.
        pub mod memory_module;
        /// Memory map module for data provided from GRUB in multiboot structure. 
        pub mod memory_map;
        /// Module that defines interface for parsing information from executable sections. 
        pub mod sections;
        /// Defines interface to GRUB's tags. 
        pub mod tags;

        /// Physical memory management.
        pub mod frames;
        /// Paging memory model management.
        pub mod paging;
        /// Structure with ownership of the P4 table in paging module
        pub mod owned_tables;
        /// Dummy page tables to map frames to virtual addresses before swap. 
        pub mod temporary_pages;
        /// Inactive page tables.
        pub mod inactive_tables;

        pub use memory_module::{MMU, InfoPointer, BootInfoHeader, MEMORY_MANAGEMENT_UNIT};
        pub use frames::AreaFrameAllocator;
        pub use stack_allocator::StackAlloc;
        
        pub use paging::{Page, Table, Entry, EntryFlags};
        pub use owned_tables::ActivePageTable;
        pub use temporary_pages::TempPage;
        pub use inactive_tables::InactivePageTable;
    }

    /// IPC and multithreading implementation.
    pub mod task_virtualization {
        /// Scheduler trait to schedule processes and threads in pairs (tasks).
        pub mod scheduler;
        /// Round Robin scheduler implementation.
        pub mod round_robin;
        /// Scheduler based on process' priority.
        pub mod priority_based_scheduling;
        
        /// Implementation of Process. A container of threads that hold their local and shared
        /// environment. Defines most important functions to run scheduled code. 
        pub mod process;
        /// Thread implementation. A simple unit that performs defined code and saves local
        /// environment before task switch.
        pub mod thread;
        /// Kernel level Join Handle for threads. Allows for synchronization without primitives.
        pub mod join_handle;
        /// Process Management Unit structure. Main structure that holds information about
        /// running/queued processes and schedules them.
        pub mod pmu;

        pub use pmu::{PMU, PROCESS_MANAGEMENT_UNIT};
        pub use process::{Process, ProcState};
        pub use thread::{Thread, ThreadFn, ThreadState};
        pub use scheduler::{Scheduler, Task};
        pub use join_handle::{JoinHandle, HandleStack};

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
        allocators::{GLOBAL_ALLOCATOR, LEAK_ALLOC, BUMP_ALLOC, NODE_ALLOC, FREE_LIST_ALLOC, BUDDY_ALLOC},
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
