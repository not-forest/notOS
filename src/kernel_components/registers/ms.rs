/// # Model Specific Registers (MSRs)
///
/// Model Specific Registers (MSRs) are control registers in the x86 system architecture
/// used for various purposes, including debugging, program execution tracing, computer
/// performance monitoring, and toggling certain CPU features.


use crate::{bitflags, VirtualAddress};
use crate::kernel_components::memory::Page;
use core::arch::asm;

/// Extended Feature Enable Register (EFER)
///
/// The EFER MSR is used to enable or disable various x86-64 CPU features. It was initially
/// added in the AMD K6 processor to allow enabling the SYSCALL/SYSRET instruction and later
/// for entering and exiting long mode.
#[derive(Debug)]
pub struct EFER; impl Msr for EFER { const MSR: u32 = 0xC0000080; }

/// FSBase MSR
///
/// This MSR contains the base address for the FS segment register.
#[derive(Debug)]
pub struct FSBase; impl Msr for FSBase { const MSR: u32 = 0xC0000100; }

/// GSBase MSR
///
/// This MSR contains the base address for the GS segment register.
#[derive(Debug)]
pub struct GSBase; impl Msr for GSBase { const MSR: u32 = 0xC0000101; }

/// KernelGSBase MSR
///
/// This MSR is essentially a buffer that gets exchanged with GS.base after a SWAPGS instruction.
/// It is typically used to separate kernel and user usage of the GS register.
#[derive(Debug)]
pub struct KernelGSBase; impl Msr for KernelGSBase { const MSR: u32 = 0xC0000102; }

/// SYSCALL register STAR (Segment Table Address Register)
///
/// STAR is an MSR used in the SYSCALL/SYSRET mechanism to control segment selectors during
/// system call transitions.
#[derive(Debug)]
pub struct Star; impl Msr for Star { const MSR: u32 = 0xC0000081; }

/// SYSCALL register LSTAR (Long Mode STAR)
///
/// LSTAR is an MSR used in the SYSCALL/SYSRET mechanism to specify the address of the system
/// call entry point when running in long mode.#[derive(Debug)]
pub struct LStar; impl Msr for LStar { const MSR: u32 = 0xC0000082; }

/// SYSCALL register SFMASK (System Call Flag Mask)
///
/// SFMASK is an MSR used in the SYSCALL/SYSRET mechanism to control the setting of EFLAGS
/// during system call transitions.
#[derive(Debug)]
pub struct SFMask; impl Msr for SFMask { const MSR: u32 = 0xC0000083; }


/// CET Configuration (User Mode)
///
/// This MSR holds control bits related to Control-Flow Enforcement Technology (CET) in user mode.
#[derive(Debug)]
pub struct UCet; impl Msr for UCet { const MSR: u32 = 0xC0000084; }

/// CET Configuration (Supervisor Mode)
///
/// This MSR holds control bits related to Control-Flow Enforcement Technology (CET) in supervisor mode.
#[derive(Debug)]
pub struct SCet; impl Msr for SCet { const MSR: u32 = 0xC0000085; }

bitflags! {
    /// Config of EFER.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub struct EFERFlags: u64 {
        /// Enables the syscall and sysret extensions.
        const SYSTEM_CALL_EXTENSIONS =          1,
        /// (!AMD K6 Only) Enables data prefetching
        const DATA_PREFETCH_ENABLE =            1 << 1,
        /// (!AMD K6 Only) Enables speculative execution write-back
        const SPECULATIVE_EWBE_DISABLE =        1 << 2,
        /// (!AMD K6 Only) Disables global write-back execution
        const GLOBAL_EWBE_DISABLE =             1 << 3,
        /// (!AMD K6 Only) Disables the level 2 cache.
        const L2_CACHE_DISABLE =                1 << 4,
        // Bits 5-7 are reserved. Read as zero.
        /// Enables long mode in the OS if the paging is on. (Cr0: 31th bit).
        const LONG_MODE_ENABLE =                1 << 8,
        /// Shows is the long mode is active or not as a flag.
        // Bit 9 is reserved.
        const LONG_MODE_ACTIVE =                1 << 10,
        /// Enables no-execute protection. It is a technology used in CPUs to segregate areas of a 
        /// virtual address space to store either data or processor instructions. An operating system
        /// with support for the NX bit may mark certain areas of an address space as non-executable.
        const NO_EXECUTE_ENABLE =               1 << 11,
        /// Enables the secure virtual machine extensions.
        const SECURE_VIRTUAL_MACHINE_ENABLE =   1 << 12,
        /// Enables the segment limit for long mode.
        const LONG_MODE_SEGMENT_LIMIT_ENABLE =  1 << 13,
        /// Enables the fxsave and fxstor instructions features. Fast FXSAVE/FXRSTOR is an optimization
        /// introduced in newer x86-64 CPUs that accelerates the context switching of the x87 FPU 
        /// (Floating-Point Unit) and SSE (Streaming SIMD Extensions) state during task switches or 
        /// thread context switches. When the FFXSR bit is set, it indicates support for this feature.
        const FAST_FXSAVE_FXSTOR =              1 << 14,
        /// Changes how the `invlpg` instruction operates on TLB entries of upper-level entries.
        const TRANSLATION_CACHE_EXTENSION =     1 << 15,
        // Bit 16 is reserved
        /// Enables the mcommit instruction.
        const MCOMMIT_ENABLE =                  1 << 17,
        /// Controls whether the WBINVD (Write-Back Invalidate) and WBNOINVD (Write-Back No Invalidate) 
        /// instructions can be interrupted by external interrupts. 
        const INTERRUPTIBLE_WB =                1 << 18,
        // Bit 19 is reserved.
        /// UAIE, or the Upper Address Ignore Enable bit, controls whether the processor ignores the 
        /// upper 32 bits of the linear address when performing address translation.
        const UPPER_ADDRESS_IGNORE_ENABLE =     1 << 20,
        /// AIBRSE, or the Automatic IBRS Enable bit, controls automatic enabling of Indirect Branch 
        /// Restricted Speculation (IBRS) when certain conditions are met. IBRS is a security feature 
        /// designed to mitigate certain types of Spectre vulnerabilities
        const AUTOMATIC_IBRS_ENABLE =           1 << 21,
        // Bits 22-63 are reserved. 
    };

    /// Config for UCET and SCET registers. They are equal for them both.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub struct XCETFlags: u64 {
        /// Enable shadow stack (SH_STK_EN)
        const SHADOW_STACK_ENABLE =                              1,
        /// Enable WRSS{D,Q}W instructions (WR_SHTK_EN)
        const SHADOW_STACK_WRITE_ENABLE =                        1 << 1,
        /// Enable indirect branch tracking (ENDBR_EN)
        const INDIRECT_BRANCH_TRACKING_ENABLE =                  1 << 2,
        /// Enable legacy treatment for indirect branch tracking (LEG_IW_EN)
        const INDIRECT_BRANCH_TRACKING_LEGACY_ENABLE =           1 << 3,
        /// Enable no-track opcode prefix for indirect branch tracking (NO_TRACK_EN)
        const INDIRECT_BRANCH_TRACKING_NO_TRACK_ENABLE =         1 << 4,
        /// Disable suppression of CET on legacy compatibility (SUPPRESS_DIS)
        const INDIRECT_BRANCH_TRACKING_LEGACY_SUPPRESS_ENABLE =  1 << 5,
        /// Enable suppression of indirect branch tracking (SUPPRESS)
        const INDIRECT_BRANCH_TRACKING_SUPPRESS_ENABLE =         1 << 10,
        /// Is IBT waiting for a branch to return? (read-only, TRACKER)
        const INDIRECT_BRANCH_TRACKING_TRACKED =                 1 << 11,
    };
}

// Model specific register trait.
/// 
/// It can be any of various control registers in the x86 system architecture used for debugging, 
/// program execution tracing, computer performance monitoring, and toggling certain CPU features.
/// 
/// Every single register that implements this trait is considered as a model specific. All 
pub trait Msr {
    const MSR: u32;
    
    /// Reads the msr register.
    /// 
    /// # Unsafe
    /// 
    /// The safety of that operation depends on the selected register and the outcome
    /// of the read.
    #[inline]
    unsafe fn read_raw() -> u64 {
        let (high, low): (u32, u32);
        unsafe {
            asm!(
                "rdmsr",
                in("ecx") Self::MSR,
                out("eax") low, out("edx") high,
                options(nomem, nostack, preserves_flags),
            );
        }
        ((high as u64) << 32) | (low as u64)
    }

    /// Writes the 64bit value to the given msr register.
    /// 
    /// # Safety
    /// 
    /// The side effects of the write completely depends on the chosen register.
    #[inline]
    unsafe fn write_raw(value: u64) {
        let low = value as u32;
            let high = (value >> 32) as u32;

            unsafe {
                asm!(
                    "wrmsr",
                    in("ecx") Self::MSR,
                    in("eax") low, in("edx") high,
                    options(nostack, preserves_flags),
                );
            }
    }
}

impl EFER {
    /// Reads the current value of EFER register
    #[inline]
    pub fn read() -> EFERFlags {
        unsafe { Self::read_raw() }.into()
    }

    /// Writes the new value values into the EFER register.
    /// 
    /// # Unsafe
    /// 
    /// Wrong flags can cause memory safety issues.
    #[inline]
    pub unsafe fn write(flags: EFERFlags) {
        let old_flags = Self::read();
        let reserved = old_flags & !EFERFlags::all();
        let new_flags = reserved | flags;

        unsafe { Self::write_raw(new_flags.into()) }
    }

    /// Enables the nxe bit. This feature is essential for enhancing system security and stability.
    /// 
    /// By enabling the NXE bit, you instruct the CPU to prevent the execution of code in memory
    /// regions that should only contain data. This helps protect against various security
    /// vulnerabilities, such as buffer overflows and stack smashing attacks, where an attacker
    /// attempts to execute arbitrary code in data segments.
    #[inline]
    pub fn enable_nxe_bit() {
        let old_flags = Self::read();
        let reserved = old_flags & !EFERFlags::all();
        let new_flags = reserved | EFERFlags::NO_EXECUTE_ENABLE;

        unsafe { Self::write_raw(new_flags.into()) }
    }
}

impl FSBase {
    /// Reads the current value of FsBase register.
    #[inline]
    pub fn read() -> VirtualAddress {
        VirtualAddress::from(unsafe { Self::read_raw() } as usize)
    }

    /// Writes the new address value into the FsBase register.
    #[inline]
    pub fn write(addr: VirtualAddress) {
        unsafe { Self::write_raw(addr as u64) };
    }
}

impl GSBase {
    /// Reads the current value of GSBase register.
    #[inline]
    pub fn read() -> VirtualAddress {
        VirtualAddress::from(unsafe { Self::read_raw() } as usize)
    }

    /// Writes the new address value into the GSBase register.
    #[inline]
    pub fn write(addr: VirtualAddress) {
        unsafe { Self::write_raw(addr as u64) };
    }
}

impl KernelGSBase {
    /// Reads the current value of KernelGSBase register.
    #[inline]
    pub fn read() -> VirtualAddress {
        VirtualAddress::from(unsafe { Self::read_raw() } as usize)
    }

    /// Writes the new address value into the KernelGSBase register.
    #[inline]
    pub fn write(addr: VirtualAddress) {
        unsafe { Self::write_raw(addr as u64) };
    }
}



impl UCet {
    /// Reads the current CET values of UCet register and the address to the legacy code page.
    #[inline]
    pub fn read() -> (XCETFlags, Page) {
        let raw_value = unsafe { Self::read_raw() };
        let cet_flags = XCETFlags::from(raw_value);
        let legacy_code_page = Page::containing_address(raw_value as usize);

        (cet_flags, legacy_code_page)
    }

    /// Writes the new CET value into the UCet register.
    #[inline]
    pub fn write(flags: XCETFlags, legacy_code_page: Page) {
        unsafe { 
            Self::write_raw(flags.bits() | legacy_code_page.start_address() as u64)
        };
    }
}

impl SCet {
    /// Reads the current CET values of SCet register and the address to the legacy code page.
    #[inline]
    pub fn read() -> (XCETFlags, Page) {
        let raw_value = unsafe { Self::read_raw() };
        let cet_flags = XCETFlags::from(raw_value);
        let legacy_code_page = Page::containing_address(raw_value as usize);

        (cet_flags, legacy_code_page)
    }

    /// Writes the new CET value into the SCet register.
    #[inline]
    pub fn write(flags: XCETFlags, legacy_code_page: Page) {
        unsafe { 
            Self::write_raw(flags.bits() | legacy_code_page.start_address() as u64)
        };
    }
}