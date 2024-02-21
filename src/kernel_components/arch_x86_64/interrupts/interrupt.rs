/// Interrupts and other procedures to the CPU. 
/// 
/// Exception catching are done with interrupt description table and handler functions.

use core::arch::asm;
use proc_macros::Iternum;

use crate::kernel_components::registers::flags::{XFLAGS, XFLAGSFlags};

/// INT vector table enum
///
/// This enum holds name for each interrupt handler which are mapped in the interrupt descriptor
/// table. It includes hardware exceptions as well as software interrupts.
#[derive(Iternum, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[allow(non_camel_case_types)]
#[repr(usize)]
pub enum InterruptVector {
    // Hardware interrupts. (exceptions)
    /// Interrupt vector number to handle division by zero.
    DIVIDE_BY_ZERO = 0x0,
    // The address 0x1 is reserved and should not be used.
    
    /// Interrupt vector number to handle division by zero. This interrupt is triggered when a division operation
    /// encounters a divisor of zero, causing a divide-by-zero error.
    NMI_INTERRUPT = 0x2,
    /// Breakpoint exception. This interrupt is typically used for debugging purposes. When encountered,
    /// it halts program execution to allow for debugging tools to inspect the program state.
    BREAKPOINT = 0x3,
    /// Overflow exception. This interrupt is triggered when an arithmetic operation results in a value that
    /// exceeds the range representable by the data type, causing an overflow condition.
    OVERFLOW = 0x4,
    /// Bounds Check exception. This interrupt is triggered when an array index or pointer exceeds the
    /// bounds of the defined range, indicating a memory access violation.
    BOUNDS_OF_RANGE_EXCEPTIONS = 0x5,
    /// Invalid Opcode exception. This interrupt occurs when the processor encounters an undefined or
    /// unrecognized instruction during execution.
    INVALID_OPCODE = 0x6,
    /// Device Not Available exception. This interrupt is raised when an attempt is made to access a hardware
    /// device that is not available or accessible at the specified address.
    DEVICE_NOT_ABAILABLE = 0x7,
    /// Double Fault exception. This interrupt occurs when an exception is encountered while processing a prior
    /// exception, indicating a severe system error that may require a system restart.
    DOUBLE_FAULT = 0x8,
    /// Coprocessor Segment Overrun exception. This interrupt is triggered when there is a segmentation fault
    /// related to coprocessor operations, typically in older x86 systems.
    COPROCESSOR_SEGMENT_OVERRUN = 0x9,
    /// Invalid Task State Segment exception. This interrupt occurs when the processor encounters an invalid
    /// Task State Segment (TSS) while attempting to switch tasks.
    INVALID_TSS = 0xa,
    /// Segment Not Present exception. This interrupt is raised when an attempt is made to access a memory
    /// segment that is marked as not present in the system's memory management tables.
    SEGMENT_NOT_PRESENT = 0xb,
    /// Stack Segment Fault exception. This interrupt occurs when there is an error related to the stack segment,
    /// such as stack underflow or stack overflow.
    STACK_SEGMENT_FAULT = 0xc,
    /// General Protection Fault exception. This interrupt is raised when an attempt is made to access protected
    /// memory or perform privileged operations without the necessary permissions.
    GENERAL_PROTECTION_FAULT = 0xd,
    /// Page Fault exception. This interrupt is raised when a memory access fails due to a page-level protection
    /// violation or when accessing a page that is not currently present in physical memory.
    PAGE_FAULT = 0xe,

    // The address 0xf is reserved and should not be used.
    
    /// x87 FPU Error exception. This interrupt occurs when there is an error during the execution of a
    /// floating-point operation using the x87 FPU (Floating Point Unit).
    X87_FPU_ERROR = 0x10,
    /// Alignment Check exception. This interrupt is triggered when an unaligned memory access is attempted,
    /// which may result in performance penalties or system instability on architectures that do not support
    /// unaligned memory accesses.
    ALIGNMENT_CHECK = 0x11,
    /// Machine Check exception. This interrupt is raised when the processor detects an unrecoverable hardware
    /// error or inconsistency, indicating a serious system failure.
    MACHINE_CHECK = 0x12,
    /// SIMD Floating Point Exception. This interrupt occurs when there is an error during the execution of
    /// SIMD (Single Instruction, Multiple Data) floating-point instructions.
    SIMD_FLOATING_POINT_EXCEPTION = 0x13,

    // Addresses 0x14 - 0x1f are reserved and must not be used.

    // Software interrupts (Mapped by PIC or APIC.)
    //
    // Those mappings are OS specific, even though those values are used most of the time.
    
    /// Mappings that are specific for PIC controller.
    PICMappings(usize),
    
    /// Mappings that are specific for APIC controller. (TODO! add the APIC controller support.)
    APICMappings(usize),

    /// Custom mappings for OS specific software interrupts.
    Custom(usize),
}

/// Enables interrupts.
#[inline(always)]
pub unsafe fn enable() {
    unsafe { asm!("sti", options(preserves_flags, nostack)) }
}

/// Disables interrupts.
#[inline(always)]
pub unsafe fn disable() {
    unsafe { asm!("cli", options(preserves_flags, nostack)) }
}

/// Checks if the interrupts are enables and returns true if they are.
#[inline(always)]
pub fn is_interrupts_enabled() -> bool {
    XFLAGSFlags::INTERRUPT_FLAG.is_in(XFLAGS::read().bits())
}

/// Does something with disabled interrupts.
/// 
/// This function is suitable for preventing deadlocks and other awful things that could be
/// caused via interrupts. This basically disables the software interrupts to occur, which is
/// timer interrupts and i/o's. It prevents the interrupt to cause undefined behavior of something
/// that should not be interrupted.
/// 
/// # Unsafe
/// 
/// This function is unsafe because it must be used only in a very short and atomic parts of
/// the OS logic. Overusing this will cause a latency in interrupts.
#[inline(always)]
pub unsafe fn with_int_disabled<F, T>(fun: F) -> T where F: FnOnce() -> T {
    let enabled = XFLAGSFlags::INTERRUPT_FLAG.is_in(XFLAGS::read().bits());

    if enabled {
        disable();
    }

    let output = fun();

    if enabled {
        enable();
    }

    output
}

/// Does something with enabled interrupts.
/// 
/// This function can be used for doing something, which must be or can be interrupted via execution.
/// 
/// # Unsafe
/// 
/// This function is unsafe because the software interrupts must be initialized properly.
#[inline(always)]
pub unsafe fn with_int_enabled<F, T>(fun: F) -> T where F: FnOnce() -> T {
    let enabled = XFLAGSFlags::INTERRUPT_FLAG.is_in(XFLAGS::read().bits());

    if !enabled {
        enable();
    }

    let output = fun();

    if !enabled {
        disable();
    }

    output
}

/// Halts the processor. This function is a better version of an infinite loop that will not
/// overuse the CPU power.
#[inline(always)]
pub fn hlt() {
    unsafe { asm!("hlt", options(nomem, nostack)) }
}

/// Cause a breakpoint.
/// 
/// This is a wrapper around a regular int3 instruction.
#[inline(always)]
#[no_mangle]
pub fn breakpoint() {
    unsafe { asm!("int3", options(nomem, nostack)) }
}

/// A simple wrapper that enables interrupts and halts the processor until the interrupt happens.
#[inline(always)]
#[no_mangle]
pub fn wait_for_interrupt() {
    unsafe {
        enable();
        hlt();       
    }
}

/// Divides a given integer by zero.
/// 
/// This function is only usable to test out the handler function, that must be called after such
/// operation. The provided integer must be u32.
#[inline(always)]
#[no_mangle]
pub unsafe fn divide_by_zero(input: u32) {
    unsafe {
        asm!(
            "mov {0:r}, rax",            // Load the input value into RAX
            "mov rdx, 0",                // Set RDX to 0 to create a 64-bit dividend
            "div rdx",                   // Divide RAX by RDX (zero)
            "mov {0:r}, rax",            // Store the result back into the input variable
            inout(reg) input => _
        );
    }
}

/// Causes an interrupt based on the provided interrupt vector.
/// 
/// This function can be usable for testing out handler functions, or causing
/// a software interrupt for personal attends.
/// 
/// # Warn
/// 
/// Causing an exception with this function is possible but it is not a recommended
/// behavior. To test out exceptions, use functions that provide to such exceptions
/// or cause them via some kind of fault or memory corruption.
/// 
/// If you still wish to cause the exception via INTn, use the unsafe version of this
/// function.
#[inline(always)]
pub fn cause_interrupt(vector_num: u8) {
    match vector_num {
        0..=31 => panic!("Accessing CPU exceptions and reserved fields are not recommended."),
        _ => unsafe{ cause_interrupt_unsafe(vector_num) },
    }
}

/// Causes an interrupt or exception based on the provided interrupt vector.
/// 
/// # Unsafe
/// 
/// This function is unsafe, because it provides support for causing an exceptions
/// which should not be caused by the software via INTn.
/// 
/// If you do not wish to call an exception or you are not sure what are you calling,
/// then use the safe version of this function instead.
#[inline(always)]
pub unsafe fn cause_interrupt_unsafe(vector_num: u8) {
    // There is no better way to write it in rust, except to create a macro, but
    // it requires a #![asm_const] to be enabled. Too much dumb things, better just
    // spam with match values.
    match vector_num {
        0 => asm!("int 0", options(nomem, preserves_flags)),
        1 => asm!("int 1", options(nomem, preserves_flags)),
        2 => asm!("int 2", options(nomem, preserves_flags)),
        3 => asm!("int 3", options(nomem, preserves_flags)),
        4 => asm!("int 4", options(nomem, preserves_flags)),
        5 => asm!("int 5", options(nomem, preserves_flags)),
        6 => asm!("int 6", options(nomem, preserves_flags)),
        7 => asm!("int 7", options(nomem, preserves_flags)),
        8 => asm!("int 8", options(nomem, preserves_flags)),
        9 => asm!("int 9", options(nomem, preserves_flags)),
        10 => asm!("int 10", options(nomem, preserves_flags)),
        11 => asm!("int 11", options(nomem, preserves_flags)),
        12 => asm!("int 12", options(nomem, preserves_flags)),
        13 => asm!("int 13", options(nomem, preserves_flags)),
        14 => asm!("int 14", options(nomem, preserves_flags)),
        15 => asm!("int 15", options(nomem, preserves_flags)),
        16 => asm!("int 16", options(nomem, preserves_flags)),
        17 => asm!("int 17", options(nomem, preserves_flags)),
        18 => asm!("int 18", options(nomem, preserves_flags)),
        19 => asm!("int 19", options(nomem, preserves_flags)),
        20 => asm!("int 20", options(nomem, preserves_flags)),
        21 => asm!("int 21", options(nomem, preserves_flags)),
        22 => asm!("int 22", options(nomem, preserves_flags)),
        23 => asm!("int 23", options(nomem, preserves_flags)),
        24 => asm!("int 24", options(nomem, preserves_flags)),
        25 => asm!("int 25", options(nomem, preserves_flags)),
        26 => asm!("int 26", options(nomem, preserves_flags)),
        27 => asm!("int 27", options(nomem, preserves_flags)),
        28 => asm!("int 28", options(nomem, preserves_flags)),
        29 => asm!("int 29", options(nomem, preserves_flags)),
        30 => asm!("int 30", options(nomem, preserves_flags)),
        31 => asm!("int 31", options(nomem, preserves_flags)),
        32 => asm!("int 32", options(nomem, preserves_flags)),
        33 => asm!("int 33", options(nomem, preserves_flags)),
        34 => asm!("int 34", options(nomem, preserves_flags)),
        35 => asm!("int 35", options(nomem, preserves_flags)),
        36 => asm!("int 36", options(nomem, preserves_flags)),
        37 => asm!("int 37", options(nomem, preserves_flags)),
        38 => asm!("int 38", options(nomem, preserves_flags)),
        39 => asm!("int 39", options(nomem, preserves_flags)),
        40 => asm!("int 40", options(nomem, preserves_flags)),
        41 => asm!("int 41", options(nomem, preserves_flags)),
        42 => asm!("int 42", options(nomem, preserves_flags)),
        43 => asm!("int 43", options(nomem, preserves_flags)),
        44 => asm!("int 44", options(nomem, preserves_flags)),
        45 => asm!("int 45", options(nomem, preserves_flags)),
        46 => asm!("int 46", options(nomem, preserves_flags)),
        47 => asm!("int 47", options(nomem, preserves_flags)),
        48 => asm!("int 48", options(nomem, preserves_flags)),
        49 => asm!("int 49", options(nomem, preserves_flags)),
        50 => asm!("int 50", options(nomem, preserves_flags)),
        51 => asm!("int 51", options(nomem, preserves_flags)),
        52 => asm!("int 52", options(nomem, preserves_flags)),
        53 => asm!("int 53", options(nomem, preserves_flags)),
        54 => asm!("int 54", options(nomem, preserves_flags)),
        55 => asm!("int 55", options(nomem, preserves_flags)),
        56 => asm!("int 56", options(nomem, preserves_flags)),
        57 => asm!("int 57", options(nomem, preserves_flags)),
        58 => asm!("int 58", options(nomem, preserves_flags)),
        59 => asm!("int 59", options(nomem, preserves_flags)),
        60 => asm!("int 60", options(nomem, preserves_flags)),
        61 => asm!("int 61", options(nomem, preserves_flags)),
        62 => asm!("int 62", options(nomem, preserves_flags)),
        63 => asm!("int 63", options(nomem, preserves_flags)),
        64 => asm!("int 64", options(nomem, preserves_flags)),
        65 => asm!("int 65", options(nomem, preserves_flags)),
        66 => asm!("int 66", options(nomem, preserves_flags)),
        67 => asm!("int 67", options(nomem, preserves_flags)),
        68 => asm!("int 68", options(nomem, preserves_flags)),
        69 => asm!("int 69", options(nomem, preserves_flags)),
        70 => asm!("int 70", options(nomem, preserves_flags)),
        71 => asm!("int 71", options(nomem, preserves_flags)),
        72 => asm!("int 72", options(nomem, preserves_flags)),
        73 => asm!("int 73", options(nomem, preserves_flags)),
        74 => asm!("int 74", options(nomem, preserves_flags)),
        75 => asm!("int 75", options(nomem, preserves_flags)),
        76 => asm!("int 76", options(nomem, preserves_flags)),
        77 => asm!("int 77", options(nomem, preserves_flags)),
        78 => asm!("int 78", options(nomem, preserves_flags)),
        79 => asm!("int 79", options(nomem, preserves_flags)),
        80 => asm!("int 80", options(nomem, preserves_flags)),
        81 => asm!("int 81", options(nomem, preserves_flags)),
        82 => asm!("int 82", options(nomem, preserves_flags)),
        83 => asm!("int 83", options(nomem, preserves_flags)),
        84 => asm!("int 84", options(nomem, preserves_flags)),
        85 => asm!("int 85", options(nomem, preserves_flags)),
        86 => asm!("int 86", options(nomem, preserves_flags)),
        87 => asm!("int 87", options(nomem, preserves_flags)),
        88 => asm!("int 88", options(nomem, preserves_flags)),
        89 => asm!("int 89", options(nomem, preserves_flags)),
        90 => asm!("int 90", options(nomem, preserves_flags)),
        91 => asm!("int 91", options(nomem, preserves_flags)),
        92 => asm!("int 92", options(nomem, preserves_flags)),
        93 => asm!("int 93", options(nomem, preserves_flags)),
        94 => asm!("int 94", options(nomem, preserves_flags)),
        95 => asm!("int 95", options(nomem, preserves_flags)),
        96 => asm!("int 96", options(nomem, preserves_flags)),
        97 => asm!("int 97", options(nomem, preserves_flags)),
        98 => asm!("int 98", options(nomem, preserves_flags)),
        99 => asm!("int 99", options(nomem, preserves_flags)),
        100 => asm!("int 100", options(nomem, preserves_flags)),
        101 => asm!("int 101", options(nomem, preserves_flags)),
        102 => asm!("int 102", options(nomem, preserves_flags)),
        103 => asm!("int 103", options(nomem, preserves_flags)),
        104 => asm!("int 104", options(nomem, preserves_flags)),
        105 => asm!("int 105", options(nomem, preserves_flags)),
        106 => asm!("int 106", options(nomem, preserves_flags)),
        107 => asm!("int 107", options(nomem, preserves_flags)),
        108 => asm!("int 108", options(nomem, preserves_flags)),
        109 => asm!("int 109", options(nomem, preserves_flags)),
        110 => asm!("int 110", options(nomem, preserves_flags)),
        111 => asm!("int 111", options(nomem, preserves_flags)),
        112 => asm!("int 112", options(nomem, preserves_flags)),
        113 => asm!("int 113", options(nomem, preserves_flags)),
        114 => asm!("int 114", options(nomem, preserves_flags)),
        115 => asm!("int 115", options(nomem, preserves_flags)),
        116 => asm!("int 116", options(nomem, preserves_flags)),
        117 => asm!("int 117", options(nomem, preserves_flags)),
        118 => asm!("int 118", options(nomem, preserves_flags)),
        119 => asm!("int 119", options(nomem, preserves_flags)),
        120 => asm!("int 120", options(nomem, preserves_flags)),
        121 => asm!("int 121", options(nomem, preserves_flags)),
        122 => asm!("int 122", options(nomem, preserves_flags)),
        123 => asm!("int 123", options(nomem, preserves_flags)),
        124 => asm!("int 124", options(nomem, preserves_flags)),
        125 => asm!("int 125", options(nomem, preserves_flags)),
        126 => asm!("int 126", options(nomem, preserves_flags)),
        127 => asm!("int 127", options(nomem, preserves_flags)),
        128 => asm!("int 128", options(nomem, preserves_flags)),
        129 => asm!("int 129", options(nomem, preserves_flags)),
        130 => asm!("int 130", options(nomem, preserves_flags)),
        131 => asm!("int 131", options(nomem, preserves_flags)),
        132 => asm!("int 132", options(nomem, preserves_flags)),
        133 => asm!("int 133", options(nomem, preserves_flags)),
        134 => asm!("int 134", options(nomem, preserves_flags)),
        135 => asm!("int 135", options(nomem, preserves_flags)),
        136 => asm!("int 136", options(nomem, preserves_flags)),
        137 => asm!("int 137", options(nomem, preserves_flags)),
        138 => asm!("int 138", options(nomem, preserves_flags)),
        139 => asm!("int 139", options(nomem, preserves_flags)),
        140 => asm!("int 140", options(nomem, preserves_flags)),
        141 => asm!("int 141", options(nomem, preserves_flags)),
        142 => asm!("int 142", options(nomem, preserves_flags)),
        143 => asm!("int 143", options(nomem, preserves_flags)),
        144 => asm!("int 144", options(nomem, preserves_flags)),
        145 => asm!("int 145", options(nomem, preserves_flags)),
        146 => asm!("int 146", options(nomem, preserves_flags)),
        147 => asm!("int 147", options(nomem, preserves_flags)),
        148 => asm!("int 148", options(nomem, preserves_flags)),
        149 => asm!("int 149", options(nomem, preserves_flags)),
        150 => asm!("int 150", options(nomem, preserves_flags)),
        151 => asm!("int 151", options(nomem, preserves_flags)),
        152 => asm!("int 152", options(nomem, preserves_flags)),
        153 => asm!("int 153", options(nomem, preserves_flags)),
        154 => asm!("int 154", options(nomem, preserves_flags)),
        155 => asm!("int 155", options(nomem, preserves_flags)),
        156 => asm!("int 156", options(nomem, preserves_flags)),
        157 => asm!("int 157", options(nomem, preserves_flags)),
        158 => asm!("int 158", options(nomem, preserves_flags)),
        159 => asm!("int 159", options(nomem, preserves_flags)),
        160 => asm!("int 160", options(nomem, preserves_flags)),
        161 => asm!("int 161", options(nomem, preserves_flags)),
        162 => asm!("int 162", options(nomem, preserves_flags)),
        163 => asm!("int 163", options(nomem, preserves_flags)),
        164 => asm!("int 164", options(nomem, preserves_flags)),
        165 => asm!("int 165", options(nomem, preserves_flags)),
        166 => asm!("int 166", options(nomem, preserves_flags)),
        167 => asm!("int 167", options(nomem, preserves_flags)),
        168 => asm!("int 168", options(nomem, preserves_flags)),
        169 => asm!("int 169", options(nomem, preserves_flags)),
        170 => asm!("int 170", options(nomem, preserves_flags)),
        171 => asm!("int 171", options(nomem, preserves_flags)),
        172 => asm!("int 172", options(nomem, preserves_flags)),
        173 => asm!("int 173", options(nomem, preserves_flags)),
        174 => asm!("int 174", options(nomem, preserves_flags)),
        175 => asm!("int 175", options(nomem, preserves_flags)),
        176 => asm!("int 176", options(nomem, preserves_flags)),
        177 => asm!("int 177", options(nomem, preserves_flags)),
        178 => asm!("int 178", options(nomem, preserves_flags)),
        179 => asm!("int 179", options(nomem, preserves_flags)),
        180 => asm!("int 180", options(nomem, preserves_flags)),
        181 => asm!("int 181", options(nomem, preserves_flags)),
        182 => asm!("int 182", options(nomem, preserves_flags)),
        183 => asm!("int 183", options(nomem, preserves_flags)),
        184 => asm!("int 184", options(nomem, preserves_flags)),
        185 => asm!("int 185", options(nomem, preserves_flags)),
        186 => asm!("int 186", options(nomem, preserves_flags)),
        187 => asm!("int 187", options(nomem, preserves_flags)),
        188 => asm!("int 188", options(nomem, preserves_flags)),
        189 => asm!("int 189", options(nomem, preserves_flags)),
        190 => asm!("int 190", options(nomem, preserves_flags)),
        191 => asm!("int 191", options(nomem, preserves_flags)),
        192 => asm!("int 192", options(nomem, preserves_flags)),
        193 => asm!("int 193", options(nomem, preserves_flags)),
        194 => asm!("int 194", options(nomem, preserves_flags)),
        195 => asm!("int 195", options(nomem, preserves_flags)),
        196 => asm!("int 196", options(nomem, preserves_flags)),
        197 => asm!("int 197", options(nomem, preserves_flags)),
        198 => asm!("int 198", options(nomem, preserves_flags)),
        199 => asm!("int 199", options(nomem, preserves_flags)),
        200 => asm!("int 200", options(nomem, preserves_flags)),
        201 => asm!("int 201", options(nomem, preserves_flags)),
        202 => asm!("int 202", options(nomem, preserves_flags)),
        203 => asm!("int 203", options(nomem, preserves_flags)),
        204 => asm!("int 204", options(nomem, preserves_flags)),
        205 => asm!("int 205", options(nomem, preserves_flags)),
        206 => asm!("int 206", options(nomem, preserves_flags)),
        207 => asm!("int 207", options(nomem, preserves_flags)),
        208 => asm!("int 208", options(nomem, preserves_flags)),
        209 => asm!("int 209", options(nomem, preserves_flags)),
        210 => asm!("int 210", options(nomem, preserves_flags)),
        211 => asm!("int 211", options(nomem, preserves_flags)),
        212 => asm!("int 212", options(nomem, preserves_flags)),
        213 => asm!("int 213", options(nomem, preserves_flags)),
        214 => asm!("int 214", options(nomem, preserves_flags)),
        215 => asm!("int 215", options(nomem, preserves_flags)),
        216 => asm!("int 216", options(nomem, preserves_flags)),
        217 => asm!("int 217", options(nomem, preserves_flags)),
        218 => asm!("int 218", options(nomem, preserves_flags)),
        219 => asm!("int 219", options(nomem, preserves_flags)),
        220 => asm!("int 220", options(nomem, preserves_flags)),
        221 => asm!("int 221", options(nomem, preserves_flags)),
        222 => asm!("int 222", options(nomem, preserves_flags)),
        223 => asm!("int 223", options(nomem, preserves_flags)),
        224 => asm!("int 224", options(nomem, preserves_flags)),
        225 => asm!("int 225", options(nomem, preserves_flags)),
        226 => asm!("int 226", options(nomem, preserves_flags)),
        227 => asm!("int 227", options(nomem, preserves_flags)),
        228 => asm!("int 228", options(nomem, preserves_flags)),
        229 => asm!("int 229", options(nomem, preserves_flags)),
        230 => asm!("int 230", options(nomem, preserves_flags)),
        231 => asm!("int 231", options(nomem, preserves_flags)),
        232 => asm!("int 232", options(nomem, preserves_flags)),
        233 => asm!("int 233", options(nomem, preserves_flags)),
        234 => asm!("int 234", options(nomem, preserves_flags)),
        235 => asm!("int 235", options(nomem, preserves_flags)),
        236 => asm!("int 236", options(nomem, preserves_flags)),
        237 => asm!("int 237", options(nomem, preserves_flags)),
        238 => asm!("int 238", options(nomem, preserves_flags)),
        239 => asm!("int 239", options(nomem, preserves_flags)),
        240 => asm!("int 240", options(nomem, preserves_flags)),
        241 => asm!("int 241", options(nomem, preserves_flags)),
        242 => asm!("int 242", options(nomem, preserves_flags)),
        243 => asm!("int 243", options(nomem, preserves_flags)),
        244 => asm!("int 244", options(nomem, preserves_flags)),
        245 => asm!("int 245", options(nomem, preserves_flags)),
        246 => asm!("int 246", options(nomem, preserves_flags)),
        247 => asm!("int 247", options(nomem, preserves_flags)),
        248 => asm!("int 248", options(nomem, preserves_flags)),
        249 => asm!("int 249", options(nomem, preserves_flags)),
        250 => asm!("int 250", options(nomem, preserves_flags)),
        251 => asm!("int 251", options(nomem, preserves_flags)),
        252 => asm!("int 252", options(nomem, preserves_flags)),
        253 => asm!("int 253", options(nomem, preserves_flags)),
        254 => asm!("int 254", options(nomem, preserves_flags)),
        255 => asm!("int 255", options(nomem, preserves_flags)),
    }
}

/// A macro that provides an easy way to implement critical sections.
///
/// This macro is just a wrapper around the with_int_disabled method, so it is not necessary to
/// always import it from this module. This macro can return values the same way as the method
/// does.
#[macro_export]
macro_rules! critical_section {
    ($fn:expr) => {
        unsafe {
            $crate::kernel_components::arch_x86_64::interrupts::with_int_disabled(|| {
                $fn()
            })
        }
    };
}
