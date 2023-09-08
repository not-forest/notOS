/// Functions to manipulate with control registers.
 
use crate::kernel_components::memory::{frames::Frame, paging::BIT_MASK};
use crate::kernel_components::instructions::TLB::Pcid;
use crate::{bitflags, VirtualAddress};
use core::arch::asm;

/// Control flags that modify a basic operations of the CPU.
/// Control Register 0 contains the address of the segment 
/// table for dynamic address translation.
#[derive(Debug)]
pub struct Cr0;

/// This register contains the Page Fault Linear Address (PFLA).
/// 
/// When a page fault occurs, the CPU will set this register to the faulting virtual address.
#[derive(Debug)]
pub struct Cr2;

/// This register contains the physical address of higher-level page table.
/// 
/// Used when virtual addressing is enabled, hence when the PG bit is set 
/// in CR0. CR3 enables the processor to translate linear addresses into 
/// physical addresses by locating the page directory and page tables for 
/// the current task.
#[derive(Debug)]
pub struct Cr3;

/// This is the extended mask register. It contains flags for various
/// arch extensions and custom support for specific processor capabilities.
/// 
/// Used in protected mode to control operations such as virtual-8086 
/// support, enabling I/O breakpoints, page size extension and machine-check
/// exceptions.
#[derive(Debug)]
pub struct Cr4;

/// CR8 is a new register accessible in 64-bit mode using the REX prefix. 
/// CR8 is used to prioritize external interrupts and is referred to as 
/// the task-priority register (TPR).
#[derive(Debug)]
pub struct Cr8;

/// XCR0, or Extended Control Register 0, is a control register which is 
/// used to toggle the storing or loading of registers related to specific 
/// CPU features using the XSAVE/XRSTOR instructions. It is also used with 
/// some features to enable or disable the processor's ability to execute 
/// their corresponding instructions. It can be changed using the privileged 
/// XSETBV read using the unprivileged XGETBV instructions.
#[derive(Debug)]
pub struct XCr0;

bitflags! {
    /// Config of Cr0.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub struct Cr0Flags: u64 {
        /// Flag to enable protected mode. If 1, system is in 
        /// protected mode, else, system is in real mode
        const ENABLE_PROTECTED_MODE =               1,
        /// Enables monitoring of coprocessor. Controls interaction 
        /// of WAIT/FWAIT instructions with TS flag in CR0
        const MONITORING_COPROCESSOR =         1 << 1,
        /// Force all x87 and MMX instructions to cause an `#NE` exception.
        const EMULATE_COPROCESSOR =            1 << 2,
        /// Automatically set to 1 on _hardware_ task switch. This flags 
        /// allows lazily saving x87/MMX/SSE instructions on hardware 
        /// context switches.
        const TASK_SWITCHED =                  1 << 3,
        /// Indicates support of 387DX math coprocessor instructions.
        const EXTENSION_TYPE =                 1 << 4,
        /// Enables the native (internal) error reporting mechanism for x87 FPU errors.
        const NUMERIC_ERROR =                  1 << 5,
        /// Controls whether supervisor-level writes to read-only pages are inhibited.
        /// When set, the CPU can't write to read-only pages when privilege level is 0.
        const WRITE_PROTECT =                  1 << 16,
        /// Enables automatic usermode alignment checking if [`RFlags::ALIGNMENT_CHECK`] is also set.
        const ALIGNMENT_MASK =                 1 << 18,
        /// Ignored, should always be unset. Globally enables/disable write-through caching.
        const NOT_WRITE_THROUGH =              1 << 29,
        /// Globally enables/disable the memory cache
        const CACHE_DISABLE =                  1 << 30,
        /// If 1, enable paging and use the § CR3 register, else disable paging.
        const PAGING =                         1 << 31,
    };

    /// Config for Cr3.
    /// 
    /// This register controls higher-level page table.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub struct Cr3Flags: u64 {
        /// Use a writethrough policy for the table (writeback otherwise.) 
        const PAGE_LEVEL_WRITETHROUGH =        1 << 3,
        /// Disable cashing for the table.
        const PAGE_LEVEL_CACHE_DISABLE =       1 << 4,
    };

    /// Config for Cr4.
    /// 
    /// Used in protected mode to control operations such as virtual-8086 support, 
    /// enabling I/O breakpoints, page size extension and machine-check exceptions.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub struct Cr4Flags: u64 {
        /// If set, enables support for the virtual interrupt flag (VIF) in virtual-8086 mode.
        const VIRTUAL_8086_MODE_EXTENSIONS =                 1,
        /// If set, enables support for the virtual interrupt flag (VIF) in protected mode.
        const PROTECTED_MODE_VIRTUAL_INTERRUPTS =       1 << 1,
        /// If set, RDTSC instruction can only be executed when in ring 0, otherwise RDTSC
        /// can be used at any privilege level.
        const TIME_STAMP_DISABLE =                      1 << 2,
        /// If set, enables debug register based breaks on I/O space access.
        const DEBUGGING_EXTENSIONS =                    1 << 3,
        /// If set, enables 32-bit paging mode to use 4 MiB huge pages in addition 
        /// to 4 KiB pages. If PAE is enabled or the processor is in x86-64 long mode 
        /// this bit is ignored.
        const PAGE_SIZE_EXTENSION =                     1 << 4,
        /// If set, changes page table layout to translate 32-bit virtual addresses 
        /// into extended 36-bit physical addresses.
        const PHYSICAL_ADDRESS_EXTENSION =              1 << 5,
        /// If set, enables machine check interrupts to occur.
        const MACHINE_CHECK_EXCEPTION =                 1 << 6,
        /// If set, address translations (PDE or PTE records) may be shared between 
        /// address spaces.
        const PAGE_GLOBAL_ENABLED =                     1 << 7,
        /// If set, RDPMC can be executed at any privilege level, else RDPMC can only be used in ring 0.
        const ENABLE_PERFORMANCE_MONITORING_COUNTER =   1 << 8,
        /// If set, enables Streaming SIMD Extensions (SSE) instructions and fast FPU save & restore.
        const OSFXSR =                                  1 << 9,
        /// If set, enables unmasked SSE exceptions.
        const OSXMMEXCPT =                              1 << 10,
        /// If set, the SGDT, SIDT, SLDT, SMSW and STR instructions cannot be executed if CPL > 0.
        const USER_MODE_INSTRUCTION_PREVENSION =        1 << 11,
        /// If set, enables 5-Level Paging.
        const LINEAR_ADDRESSES_57BIT =                  1 << 12,
        /// See Intel VT-x x86 virtualization for info on this one.
        const VIRTUAL_MACHINE_EXTENSIONS_ENABLE =       1 << 13,
        /// See Trusted Execution Technology for info on this one.
        const SAFER_MODE_EXTENSIONS_ENABLE =            1 << 14,
        // The 15th bit is reserved.
        /// If set, enables the instructions RDFSBASE, RDGSBASE, 
        /// WRFSBASE, and WRGSBASE.
        const FSGSBASE_ENABLE =                         1 << 16,
        /// If set, enables process-context identifiers (PCIDs).
        const PCID_ENABLE =                             1 << 17,
        /// XSAVE and Processor Extended States Enable
        const OSXSAVE =                                 1 << 18,
        /// If set, enables the AES Key Locker instructions.
        const KEY_LOCKER_ENABLE =                       1 << 19,
        /// If set, execution of code in a higher ring generates a fault.
        const SMEP =                                    1 << 20,
        /// If set, access of data in a higher ring generates a fault.
        const SMAP =                                    1 << 21,
        /// See Intel 64 and IA-32 Architectures Software Developer's Manual.
        const PROTECTION_KEY_ENABLE =                   1 << 22,
        /// If set, enables control-flow enforcement technology.
        const CONTROL_FLOW_ENFORCEMENT_TECHNOLOGY =     1 << 23,
        /// If set, each supervisor-mode linear address is associated with a protection key when 
        /// 4-level or 5-level paging is in use.
        const PKS_ENABLE =                              1 << 24,
        /// If set, enables user-mode inter-processor interrupts and their associated instructions and data structures.
        const USER_INTERRUPTS_ENABLE =                  1 << 25,
    };

    /// Config for Cr8
    /// 
    /// System software can use the TPR register to temporarily block low-priority interrupts from interrupting a high-priority task. 
    /// This is accomplished by loading TPR with a value corresponding to the highest-priority interrupt that is to be blocked. For example, 
    /// loading TPR with a value of 9 (1001b) blocks all interrupts with a priority class of 9 or less, while allowing all interrupts with 
    /// a priority class of 10 or more to be recognized. Loading TPR with 0 enables all external interrupts. Loading TPR with 15 (1111b) 
    /// disables all external interrupts.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub struct Cr8Flags: u8 {
        const BIT_ZERO =  1,
        const BIT_ONE =   1 << 1,
        const BIT_TWO =   1 << 2,
        const BIT_THREE = 1 << 3,
    };
}

impl Cr0 {
    /// Read the current state of Cr0 register
    #[inline]
    pub fn read() -> Cr0Flags {
        let value: u64;

        unsafe {
            asm!("mov {}, cr0", out(reg) value, options(nomem, nostack, preserves_flags));
        }

        value.into()
    }

    /// Write new flags into Cr0 register
    /// 
    /// # Safety
    /// 
    /// This function is unsafe because it's possible to violate memory
    /// safety through it.
    #[inline]
    pub unsafe fn write(flags: Cr0Flags) {
        let old_flags = Self::read();
        let reserved = old_flags & !Cr0Flags::all();
        let new_flags = reserved | flags;
        
        unsafe {
            asm!("mov cr0, {}", in(reg) u64::from(new_flags), options(nostack, preserves_flags));
        }
    }

    /// Enables write protect bit.
    /// 
    /// CPU ignores this bit in kernel mode by default. To enable write protection 
    /// for the kernel, WRITE_PROTECT bit must be set in CR0 register.
    pub fn enable_write_protect_bit() {
        unsafe { Self::write(Cr0Flags::default() | Cr0Flags::WRITE_PROTECT) }
    }
}

impl Default for Cr0Flags {
    fn default() -> Self {
        use Cr0Flags::*;
        ENABLE_PROTECTED_MODE |
        EXTENSION_TYPE        |
        PAGING
    }
}

impl Cr2 {
    /// Read the current page fault linear address from the CR2 register.
    #[inline]
    pub fn read() -> VirtualAddress {
        let value: usize;

        unsafe {
            asm!("mov {}, cr2", out(reg) value, options(nomem, nostack, preserves_flags));
        }
        
        value
    }
}

impl Cr3 {
    /// Reads the current P4 table address from the CR3 register.
    /// 
    /// Returns a physical 'Frame' and the corresponding bitflags.
    #[inline]
    pub fn read() -> (Frame, Cr3Flags) {
        let (frame, value) = Self::inner_read();
        let flags = Cr3Flags::from_bits_truncate(value as u64).into();
        (frame, flags)
    }

    /// Write a new P4 table address into the CR3 register via bitflags.
    ///
    /// # Safety
    ///
    /// Changing the level 4 page table is unsafe, because it's possible to violate memory safety by
    /// changing the page mapping.
    #[inline]
    pub unsafe fn write(frame: Frame, flags: Cr3Flags) {
        unsafe { Self::inner_write(frame, flags) }
    }

    /// Write a new P4 table address into the CR3 register via PCID.
    ///
    /// # Safety
    ///
    /// Changing the level 4 page table is unsafe, because it's possible to violate memory safety by
    /// changing the page mapping.
    pub unsafe fn write_pcid(frame: Frame, pcid: Pcid) {
        unsafe { Self::inner_write(frame, (pcid.get() as u64).into()) }
    }

    /// Read the current P4 table address from the CR3 register along with PCID.
    #[inline]
    pub unsafe fn read_pcid() -> (Frame, Pcid) {
        let (frame, value) = Self::inner_read();
        (frame, Pcid::new(value).unwrap())
    }

    #[inline]
    fn inner_read() -> (Frame, u16) {
        let value: usize;
        
        unsafe {
            asm!("mov {}, cr3", out(reg) value, options(nomem, nostack, preserves_flags));
        }

        let addr = value & BIT_MASK;
        let frame = Frame::info_address(addr);
        
        (frame, (value & 0xFFF) as u16)
    }

    #[inline]
    unsafe fn inner_write(frame: Frame, flags: Cr3Flags) {
        let addr = frame.start_address();
        let value = addr as u64 | flags.bits();

        unsafe {
            asm!("mov cr3, {}", in(reg) value, options(nostack, preserves_flags));
        }
    }
}

impl Cr4 {
    /// Read the current state of Cr0 register
    pub fn read() -> Cr4Flags {
        let value: u64;
        
        unsafe {
            asm!("mov {}, cr4", out(reg) value, options(nomem, nostack, preserves_flags));
        }

        value.into()
    }

    /// Write flags to Cr4.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it's possible to violate memory
    /// safety through it, e.g. by overwriting the physical address extension
    /// flag.
    #[inline]
    pub unsafe fn write(flags: Cr4Flags) {
        let old_flags = Self::read();
        let reserved = old_flags & !Cr4Flags::all();
        let new_flags = reserved | flags;

        unsafe {
            asm!("mov cr4, {}", in(reg) u64::from(new_flags), options(nostack, preserves_flags));
        }
    }
}

impl Default for Cr4Flags {
    fn default() -> Self {
        Self::PHYSICAL_ADDRESS_EXTENSION
    }
}

impl Cr8 {
    /// Read external interrupts priorities as flags.
    #[inline]
    pub fn read() -> Cr8Flags {
        let value: u64;
        
        unsafe {
            asm!("mov {}, cr8", out(reg) value, options(nomem, nostack, preserves_flags));
        }

        (value as u8).into()
    }

    /// Write external interrupts priorities to Cr8 register.
    ///
    /// # Safety
    ///
    /// System software can use the TPR register to temporarily block low-priority 
    /// interrupts from interrupting a high-priority task. This can lead to certain
    /// problems with latency if not used right.
    #[inline]
    pub fn write(flags: Cr8Flags) {
        let old_flags = Self::read();
        let reserved = old_flags & !Cr8Flags::all();
        let new_flags = reserved | flags;

        unsafe {
            asm!("mov cr8, {}", in(reg) u64::from(u8::from(new_flags)), options(nostack, preserves_flags));
        }
    }

    /// Sets the priority to Cr8 register
    /// 
    /// The AMD64 architecture allows software to define up to 15 external interrupt-priority 
    /// classes. Priority classes are numbered from 1 to 15, with priority-class 1 being 
    /// the lowest and priority-class 15 the highest. 
    /// 
    /// For example, loading TPR with a value of 9 (1001b) blocks all interrupts with a priority 
    /// class of 9 or less, while allowing all interrupts with a priority class of 10 or more to 
    /// be recognized. Loading TPR with 0 enables all external interrupts. Loading TPR with 15 
    /// (1111b) disables all external interrupts.
    /// 
    /// # Panics
    /// 
    /// This function will panic, if the input is larger than 15.
    #[inline]
    pub fn set_priority(amount: u8) {
        assert!(amount <= 15, "The input must be smaller or equal to 15.");
        Self::write(amount.into());
    }
}