/// Global Descriptor Table.
/// 
/// The Global Descriptor Table (GDT) is a binary data structure specific to the IA-32 and 
/// x86-64 architectures. It contains entries telling the CPU about memory segments.
/// 
/// The GDT structure is:
/// - Exist only as a one single structure in the system.
/// - Contain all the OS code and data.
/// - Available to all tasks.

pub use crate::kernel_components::registers::segment_regs::SegmentSelector;
use crate::kernel_components::arch_x86_64::{
    PrivilegeLevel,
    DTPointer,
    descriptor_table::{lgdt, sgdt},
};
use crate::{bitflags, single, VirtualAddress};
use super::task_state_segment::{TSS_SIZE, TSS};
use core::ops::{Deref, Index};
use core::arch::asm;
use core::mem;

/// A static instance of a Global Descriptor Table.
/// 
/// The table itself is a static mutable and is not hidden behind any synchronization primitive,
/// therefore it must be used with caution. No default values, except the first empty instance
/// is predefined on initialization.
single!{
    pub mut GLOBAL_DESCRIPTOR_TABLE: GDT = GDT::new();
}

/// Global Descriptor Table (GDT) implementation.
/// 
/// The GDT is pointed to by the value in the GDTR register. This is loaded using the 
/// LGDT assembly instruction, whose argument is a pointer to a GDT Descriptor structure.
/// The table has a constant length of 8 entries, where the first one must always be null.
#[derive(Debug, Clone)]
pub struct GDT {
    // Descriptors are written as a u64 value, because there is no way to write a 4-bit value
    // in Rust. The structure is complicated and cannot be written packed.
    table: [u64; 8],
    len: usize,
}

impl GDT {
    /// Create a new instance of GDT (empty gdt).
    /// 
    /// # Length
    /// 
    /// Upon creation it is already 1 entry long. This is because the first null entry must be
    /// always null by default, therefor it is initiated like so.
    #[inline]
    pub const fn new() -> Self {
        Self { 
            table: [0; 8],
            len: 1,
        }
    }

    /// Adds the given segment descriptor to the GDT, returning the segment selector.
    ///
    /// # Warn
    /// 
    /// This function is not thread safe.
    /// 
    /// # Panics
    /// 
    /// Panics if the GDT is full.
    #[inline]
    pub fn push(&mut self, entry: SegmentDescriptor) -> SegmentSelector {
        let index = match entry {
            SegmentDescriptor::Based(low, high) => {
                assert!(self.len < 7, "This kind of segment descriptor cannot fit the table. Descriptors with base address take two positions in the GDT table.");

                let index = self._inner_push(low);
                self._inner_push(high);
                index
            },
            SegmentDescriptor::Baseless(low) => {
                assert!(self.len < 8, "The GDT is full. The table can take up to 8 entries, where the first one is always the NULL entry.");

                self._inner_push(low)
            }
        };

        SegmentSelector::new(index as u16, false, entry.get_privilege_level())
    }

    /// Returns the current table as a 'DTPointer'.
    #[inline]
    pub fn as_dt_ptr(&self) -> DTPointer {
        DTPointer {
            addr: self.table.as_ptr() as u64,
            size: (self.len * mem::size_of::<u64>() - 1) as u16,
        }
    }

    /// Returns the current table from the 'DTPointer'.
    #[inline]
    pub fn from_dt_ptr(dt_ptr: DTPointer) -> Option<&'static GDT> {
        unsafe { (dt_ptr.addr as *const Self).as_ref() }
    }

    /// Loads the GDT to the CPU.
    /// 
    /// The lifetime of the table must be static to provide safety.
    #[inline]
    pub fn load_table(&'static self) {
        unsafe { lgdt(&self.as_dt_ptr()) };
    }

    /// Reads the current table value from the CPU.
    #[inline]
    pub fn get_current_table() -> DTPointer {
        sgdt()
    }

    /// Initiates a flat setup for long mode.
    /// 
    /// ## Info
    /// 
    /// Automatically fills the GDT with the needed data for a flat setup:
    /// 
    /// Paging memory model is strictly enforced in Long Mode, as the base and limit values 
    /// are ignored. In this scenario, the only Segment Descriptors necessary are the Null 
    /// Descriptor, and one descriptor for each combination of privilege level, segment type, 
    /// and execution mode desired, as well as system descriptors. 
    /// 
    /// Usually this will consist of the one code and one data segment for kernel and user mode,
    /// and a Task State Segment.
    /// 
    /// This setup requires you to have the reference to TSS struct, because it does add the task
    /// state segment descriptor. One extra slot can be used for other descriptors.
    pub fn flat_setup(tss_ref: &'static TSS) -> Self {
        let mut gdt = Self::new();

        gdt.push(SegmentDescriptor::KERNEL_MODE_CODE_SEGMENT_64);
        gdt.push(SegmentDescriptor::KERNEL_MODE_DATA_SEGMENT);
        gdt.push(SegmentDescriptor::USER_MODE_CODE_SEGMENT_64);
        gdt.push(SegmentDescriptor::USER_MODE_DATA_SEGMENT);
        gdt.push(SegmentDescriptor::tss_segment_descriptor(tss_ref));
        
        gdt
    }

    /// Rewrites the current GDT to some pre-made value.
    pub fn reinit(&mut self, gdt: GDT) {
        *self = gdt
    }

    fn _inner_push(&mut self, entry: u64) -> usize {
        let index = self.len;
        self.table[index] = entry;
        self.len += 1;

        index
    }
}

impl Index<usize> for GDT {
    type Output = u64;

    fn index(&self, index: usize) -> &Self::Output {
        &self.table[index]
    }
}

/// A segment descriptor struct, which is a single entry value inside the GDT. These are a 
/// binary data structure that tells the CPU the attributes of a given segment.
/// 
/// The structure of the descriptor is large and complex. Because of that, it is divided
/// into it's main sections: Base, Limit, Access byte, Flags.
/// 
/// # Types
/// 
/// Those "types" are just a separation for two different variables that can be inserted into
/// the GDT table. Segments that do not have a base address, do not have to take two positions\
/// in the table. Meanwhile the TSS or LDT requires the full Long Mode 64-bit address to point
/// for the struct itself. (Only in 64-bit).
/// 
/// ## Base
/// 
/// A 32-bit value containing the linear address where the segment begins. Ignored in Long Mode
/// except for FS and GS segment registers.
/// 
/// ## Limit
/// 
/// A 20-bit value, tells the maximum addressable unit, either in 1 byte units, or in 
/// 4KiB pages. Hence, if you choose page granularity and set the Limit value to 0xFFFFF
/// the segment will span the full 4 GiB address space in 32-bit mode.
/// 
/// But in 64-bit mode, the Base and Limit values are ignored, each descriptor covers the 
/// entire linear address space regardless of what they are set to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SegmentDescriptor {
    /// Descriptor for data and code segments
    /// 
    /// Those segments do not need the base address, so they can fit into one GDT entry.
    Baseless(u64),
    /// Descriptor for segments like TSS or LDT. They do require an extra 64-bit for addressing
    /// the struct itself.
    Based(u64, u64),
}

impl SegmentDescriptor {
    /// Null descriptor.
    /// 
    /// It is never referenced by the processor, and should always contain no data. Certain
    /// emulators, like Bochs, will complain about limit exceptions if you do not have one 
    /// present. Some use this descriptor to store a pointer to the GDT itself (to use with 
    /// the LGDT instruction). The null descriptor is 8 bytes wide and the pointer is 6 bytes
    /// wide so it might just be the perfect place for this.
    pub const NULL: Self = Self::custom(
        0,
        0,
        0x00000000, 
        0x00, 
        0x0
    );

    pub const KERNEL_MODE_CODE_SEGMENT_32: Self = Self::custom(
        0,
        0,
        0xFFFFF, 
        0x9B,
        0xC
    );

    pub const KERNEL_MODE_CODE_SEGMENT_64: Self = Self::custom(
        0,
        0,
        0xFFFFF, 
        0x9B,
        0xA
    );

    pub const USER_MODE_CODE_SEGMENT_32: Self = Self::custom(
        0,
        0,
        0xFFFFF, 
        0xFB,
        0xC
    );

    pub const USER_MODE_CODE_SEGMENT_64: Self = Self::custom(
        0,
        0,
        0xFFFFF, 
        0xFB,
        0xA
    );

    pub const KERNEL_MODE_DATA_SEGMENT: Self = Self::custom(
        0,
        0,
        0xFFFFF, 
        0x93,
        0xC
    );

    pub const USER_MODE_DATA_SEGMENT: Self = Self::custom(
        0,
        0, 
        0xFFFFF, 
        0xF3,
        0xC
    );

    /// Creates a new instance of 'SegmentDescriptor'.
    /// 
    /// 32-bit is a base address used in protected mode. 64-bit base must be used only for
    /// TSS and LDT segments. 
    #[inline]
    pub const fn custom(base32: u32, base64: u64, limit: u32, access_byte: u8, flags: u8) -> Self {
        let base32 = DescriptorBase::new(base32);
        let limit = DescriptorLimit::new(limit);
        let access_byte = DescriptorAccessByte::new(access_byte);
        let flags = DescriptorFlags::new(flags);
        
        if base64 != 0 {
            SegmentDescriptor::Based(
                Self::format_bits(base32, limit, access_byte, flags), 
                base64,
            )
        } else {
            SegmentDescriptor::Baseless(
                Self::format_bits(base32, limit, access_byte, flags),
            )
        }
    }

    /// Returns a 'SegmentDescriptor' from the provided values.
    ///
    /// If you are not sure how the segment descriptor structure looks like in the x86 architecture,
    /// it is better to create it manually with custom() method.
    /// 
    /// The low bits are the informative part of the descriptor, which contains access byte,
    /// flags, limit and unused 32-bit base and limit.
    /// 
    /// The high bits are the base address used in Long Mode.
    #[inline]
    pub const fn from_bits(low_bits: u64, high_bits: u64) -> Self {
        if high_bits != 0 {
            SegmentDescriptor::Based(low_bits, high_bits)
        } else {
            SegmentDescriptor::Baseless(low_bits)
        }
    }

    /// Returns the value of the segment descriptor as u128.
    /// 
    /// If the descriptor is baseless, the high bits will be just zeroes.
    #[inline]
    pub const fn as_u128(&self) -> u128 {
        match self {
            SegmentDescriptor::Based(low_bits, high_bits) => {
                *low_bits as u128 | ((*high_bits as u128) << 64)
            },
            SegmentDescriptor::Baseless(low_bits) => *low_bits as u128,
        }
    }

    /// Returns a segment descriptor as a u64 value. (LOW BITS ONLY)
    ///
    /// This function will return a valid response even after changing the flags within.
    #[inline]
    pub const fn format_bits(
        base32: DescriptorBase, 
        limit: DescriptorLimit, 
        access_byte: DescriptorAccessByte, 
        flags: DescriptorFlags,
    ) -> u64 {
        let limit_0_15 = limit.bits() & DescriptorLimit::LIMIT_0_15.bits();
        let limit_16_19 = (limit.bits() & DescriptorLimit::LIMIT_16_19.bits()) >> 16;
        let base_0_15 = base32.bits() & DescriptorBase::BASE_0_15.bits();
        let base_16_23 = (base32.bits() & DescriptorBase::BASE_16_23.bits()) >> 16;
        let base_24_31 = (base32.bits() & DescriptorBase::BASE_24_31.bits()) >> 24;
        let access_byte = access_byte.bits();
        let flags = flags.bits();
        
        // All of those ugly bits shifting is a side effect of making the segment descriptor
        // structure easier to read.
        limit_0_15 as u64          |
        (base_0_15 as u64) <<   16 |
        (base_16_23 as u64) <<  32 |
        (access_byte as u64) << 40 |
        (limit_16_19 as u64) << 48 |
        (flags as u64) <<       52 |
        (base_24_31 as u64) <<  56 
    }

    /// Returns a privilege level of the segment.
    #[inline]
    pub fn get_privilege_level(&self) -> PrivilegeLevel {
        let mask = (DescriptorAccessByte::DESCRIPTOR_PRIVILEGE_LEVEL.bits() as u64) << 40;

        match self {
            SegmentDescriptor::Based(low, high) => {
                let privilege = (low & mask) >> 45;
                PrivilegeLevel::from_u8(privilege as u8) 
            },
            SegmentDescriptor::Baseless(low) => {
                let privilege = (low & mask) >> 45;
                PrivilegeLevel::from_u8(privilege as u8) 
            }
        }
    }

    /// Creates a new TSS descriptor. 
    #[inline]
    pub fn tss_segment_descriptor(tss_ref: &'static TSS) -> Self {
        let ptr = tss_ref as *const TSS as u64;
        SegmentDescriptor::Based(
            Self::format_bits(
                DescriptorBase::new(ptr as u32),
                DescriptorLimit::new(TSS_SIZE - 1),
                DescriptorAccessByte::new(0x89),
                DescriptorFlags::new(0x0),
            ), 
            ptr >> 32,
        )
    }
}

bitflags! {
    /// Descriptor limit.
    /// 
    /// A 20-bit value, tells the maximum addressable unit, either in 1 byte units, or in 
    /// 4KiB pages. Hence, if you choose page granularity and set the Limit value to 
    /// 0xFFFFF the segment will span the full 4 GiB address space in 32-bit mode.
    /// 
    /// # Ignored
    /// 
    /// Ignored in 64-bit mode.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct DescriptorLimit: u32 {
        const LIMIT_0_15 = 0xFFFF,
        const LIMIT_16_19 = 0xF0000,
    };

    /// Description's base is a 32bit value that contain the linear address where the
    /// segment begins.
    /// 
    /// # Ignored
    /// 
    /// Ignored in 64-bit mode, except for FS, GS.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct DescriptorBase: u32 {
        const BASE_0_15 = 0xFFFF,
        const BASE_16_23 = 0xFF0000,
        const BASE_24_31 = 0xFF000000,
    };

    /// Access byte.
    /// 
    /// For system segments, such as those defining a Task State Segment or Local Descriptor 
    /// Table, the format of the Access Byte differs slightly, in order to define different 
    /// types of system segments rather than code and data segments.
    /// 
    /// First 4 bits define the type of a system segment. Possible outcomes are:
    /// 
    /// ## 32-bit protected mode:
    /// 
    /// 0x1: 16-bit TSS (Available)
    /// 0x2: LDT
    /// 0x3: 16-bit TSS (Busy)
    /// 0x9: 32-bit TSS (Available)
    /// 0xB: 32-bit TSS (Busy)
    /// 
    /// # 64-bit long-mode:
    /// 
    /// 0x2: LDT
    /// 0x9: 64-bit TSS (Available)
    /// 0xB: 64-bit TSS (Busy)
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct DescriptorAccessByte: u8 {
        /// Accessed bit. The CPU will set it when the segment is accessed unless set in
        /// advance. This means that in case the GDT descriptor is stored in read only pages
        /// and this bit is set to 0, the CPU trying to set this bit will trigger a page fault.
        /// Best left set to 1 unless otherwise needed.
        const ACCESSED_BIT = 1,
        /// Works differently for different segment types.
        /// 
        /// ## Code segments
        /// 
        /// Readable bit. If clear, read access for this segment is not allowed. If set read
        /// access is allowed. Write access is never allowed for code segments.
        /// 
        /// ## Data segments
        /// 
        /// Writeable bit. If clear, write access for this segment is not allowed. If set 
        /// write access is allowed. Read access is always allowed for data segments.
        const READABLE_WRITABLE_BIT = 1 << 1,
        /// Works differently for different segment types.alloc
        /// 
        /// ## Code segments
        /// 
        /// For code segments it is a conforming bit. If clear code in this segment can only
        /// be executed from the ring set in DPL.
        /// 
        /// If set code in this segment can be executed from an equal or lower privilege level. 
        /// 
        /// For example, code in ring 3 can far-jump to conforming code in a ring 2 segment. 
        /// 
        /// The DPL field represent the highest privilege level that is allowed to execute the 
        /// segment.
        /// 
        /// For example, code in ring 0 cannot far-jump to a conforming code segment where DPL 
        /// is 2, while code in ring 2 and 3 can. Note that the privilege level remains the same,
        /// ie. a far-jump from ring 3 to a segment with a DPL of 2 remains in ring 3 after the jump.
        /// 
        /// ## Data segments
        /// 
        /// For data segments it is a direction bit. If clear the segment grows up. 
        /// If set the segment grows down, ie. the Offset has to be greater than the Limit.
        const DIRECTION_BIT_CONFORMING_BIT = 1 << 2,
        /// Executable bit. If clear the descriptor defines a data segment. If set it defines a code
        /// segment which can be executed from.
        const EXECUTABLE_BIT = 1 << 3,
        /// Descriptor type bit or system segment bit. It defines the system segment (code segment/data segment).
        const DESCRIPTOR_TYPE_BIT = 1 << 4,
        /// Description privilege level.
        /// 
        /// Two bits that represents four possible privilege levels.
        const DESCRIPTOR_PRIVILEGE_LEVEL = 3 << 5,
        /// Present bit allows an entry to refer to a valid segment. It must be set 1 for any
        /// valid segment.
        const PRESENT_BIT = 1 << 7,
    };
    
    /// Descriptor flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct DescriptorFlags: u8 {
        // The first bit (0) is reserved
        /// Long-mode code flag. If set, the descriptor defines a 64-bit code segment. ]
        /// When set, DB should always be clear. For any other type of segment 
        /// (other code types or any data segment), it should be clear.
        const LONG_MODE_CODE_FLAG = 1 << 1,
        /// Size flag (Descriptor Bits). If unset, the descriptor defines a 16-bit protected 
        /// mode segment. If set it defines a 32-bit protected mode segment.
        /// 
        /// A GDT can have both 16-bit and 32-bit selectors at once.
        const DESCRIPTOR_BIT_SIZE = 1 << 2,
        /// Granularity flag, indicates the size the Limit value is scaled by. If clear,
        /// the Limit is in 1 Byte blocks (byte granularity). If set, the Limit is in 4 
        /// KiB blocks (page granularity).
        const GRANULARITY_FLAG =    1 << 3,
    };
}

impl DescriptorBase {
    /// Returns a set of bits as a 'DescriptionBase'.
    pub const fn new(value: u32) -> Self {
        DescriptorBase::Custom(value)
    }
}

impl DescriptorLimit {
    /// Returns a set of bits as a 'DescriptorLimit'.
    pub const fn new(value: u32) -> Self {
        DescriptorLimit::Custom(value)
    }
}

impl DescriptorAccessByte {
    /// Returns a set of bits as a 'DescriptorAccessByte'.
    pub const fn new(value: u8) -> Self {
        DescriptorAccessByte::Custom(value)
    }
}

impl DescriptorFlags {
    /// Returns a set of bits as a 'DescriptorFlags'.
    pub const fn new(value: u8) -> Self {
        DescriptorFlags::Custom(value)
    }
}

#[test_case]
fn initiating_flat_setup() {
    use crate::kernel_components::arch_x86_64::segmentation::{TSS, GDT};
    use crate::single;
    
    static TASK_STATE_SEGMENT: TSS = TSS::new();

    single! {
        pub GLOBAL_DESC_TABLE: GDT = GDT::flat_setup(&TASK_STATE_SEGMENT);
    }

    GLOBAL_DESC_TABLE.load_table();
}
