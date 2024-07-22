// Memory module for memory management. This is the entry point of memory functions and structs. 

use core::alloc::Allocator;
use core::mem::{self, size_of, MaybeUninit};
use core::fmt::{Debug, Display};
use core::error::Error;
use core::sync::atomic::{AtomicBool, Ordering};

use crate::kernel_components::arch_x86_64::acpi::rsdt::SDTPointer;
use crate::kernel_components::arch_x86_64::{
    segmentation::TSS,
    acpi::rsdt::{ACPITagOld, ACPITagNew},
    acpi::{RSDT, XSDT, acpi::ACPISDTHeader},
};
use crate::kernel_components::memory::frames::PAGE_SIZE;
use crate::{VirtualAddress, PhysicalAddress, println};
use crate::single;

use super::frames::FrameIter;
use super::owned_tables::InnerMapper;
use super::EntryFlags;
use super::{
    Page, ActivePageTable,
    tags::{EndTag, TagTrait, TagIter}, 
    memory_map::MemoryMapTag,
    sections::{SectionsTag, SectionIter}, 
    frames::{Frame, FrameAlloc, AreaFrameAllocator}, 
    temporary_pages::TempPage, 
    inactive_tables::InactivePageTable,
    stack_allocator::{Stack, StackAlloc},
};

type MMUResult = Result<(), MemError>;

/// A Memory Management Unit.
/// 
/// This structure provides all necessary functions, related to memory management
/// and contain the most important memory related info.
#[derive(Debug)]
pub struct MMU {
    /// Grub's multiboot info structure.
    info_pointer: InfoPointer<'static>,
    /// Active table 
    active_table: Option<ActivePageTable>,
    /// Area frame allocator instance, for allocating frames.
    frame_allocator: AreaFrameAllocator,
    /// Stack allocator instance, which allocates custom OS stacks.
    stack_allocator: StackAlloc,

    /// Amount of allocated frames.
    frames_allocated: usize,
    /// Mark that makes initialization possible only once.
    is_mem_init: AtomicBool,
}

/// A static MMU instance.
/// 
/// Note that it does not have a proper multiboot structure pointer, and must be initialized with
/// init() method.
single! {
    pub mut MEMORY_MANAGEMENT_UNIT: MMU = unsafe { MMU::new() };
}

impl MMU {

    /// Creates a new instance of MMU without the pointer.
    /// 
    /// This function does not remap the kernel and only creates an instance of this unit.
    #[inline]
    pub fn new() -> Self {
        use crate::kernel_components::memory::{
            self, 
            allocators::GLOBAL_ALLOCATOR,
            EntryFlags,
        };

        unsafe {
            #[allow(invalid_value)]
            Self {
                info_pointer: MaybeUninit::uninit().assume_init(),
                active_table: None,
                
                frame_allocator: MaybeUninit::uninit().assume_init(),
                stack_allocator: MaybeUninit::uninit().assume_init(),
    
                frames_allocated: 0,
                is_mem_init: AtomicBool::new(false),
            }
        }
    }

    /// Creates a new instance of MMU with given info pointer.
    /// 
    /// Provide the info pointer to _start() function the kernel. This function
    /// creates a new instance of MMU and also remaps the kernel, initialize the
    /// paging, and sets the active table.
    /// 
    /// # Note
    /// 
    /// This function only does memory based operations without changing states of any register or
    /// global state. Any specific properties of registers related to memory must be set manually
    /// via kernel_components::registers::control.
    #[inline]
    pub fn new_init(multiboot_information_address: usize) -> Self {
        let boot_info = unsafe { 
            InfoPointer::load(
                multiboot_information_address as *const BootInfoHeader 
            ) 
        }.unwrap();

        use crate::kernel_components::arch_x86_64::acpi::{XSDT, RSDT};
        use crate::kernel_components::memory::{
            self,
            allocators::GLOBAL_ALLOCATOR,
            EntryFlags,
        };

        // Getting the MMAP tag.
        let memory_map_tag = boot_info.memory_map_tag()
        .expect("Memory map tag required.");

        // Getting kernel boundaries
        let kernel_start = boot_info.kstart();
        let kernel_end = boot_info.kend();
        // Getting multiboot2 boundaries.
        let multiboot_start = boot_info.mstart();
        let multiboot_end = boot_info.mend();
        // Getting heap boundaties.
        let heap_start = unsafe { GLOBAL_ALLOCATOR.heap_addr };
        let heap_end = heap_start + unsafe{ GLOBAL_ALLOCATOR.arena_size };

        let mut frame_allocator = AreaFrameAllocator::new(
            kernel_start as usize, 
            kernel_end as usize, 
            multiboot_start, 
            multiboot_end,
            memory_map_tag.memory_map_iter(),
        );

        #[cfg(debug_assertions)] { println!("Remapping start"); }
        
        // remaping the kernel
        let mut active_table = MMU::remap_kernel(&mut frame_allocator, &boot_info);
        #[cfg(debug_assertions)] { println!("Remapping complete!"); }

        let heap_start_page = Page::containing_address(heap_start);
        let heap_end_page = Page::containing_address(heap_end);

        #[cfg(debug_assertions)] { println!("Mapping the heap pages."); }

        for page in Page::range_inclusive(heap_start_page, heap_end_page) {
            active_table.map(page, EntryFlags::WRITABLE, &mut frame_allocator);
            #[cfg(debug_assertions)]
            println!(crate::Color::LIGHTGRAY; "Mapping page at address {:#x}", page.start_address());
        }   
        #[cfg(debug_assertions)] { println!("Mapping complete."); }

        let stack_allocator = StackAlloc::new(heap_end_page + 1);

        Self {
            info_pointer: boot_info,
            active_table: Some(active_table),
            frame_allocator: frame_allocator,
            stack_allocator: stack_allocator,

            frames_allocated: (multiboot_end - multiboot_start) / PAGE_SIZE,
            is_mem_init: AtomicBool::new(true),
        }
    }

    /// Initiates the memory.
    /// 
    /// This function remaps the kernel, initialize the stack and sets active table
    /// in the structure.
    /// 
    /// 
    /// # Panics
    /// 
    /// Panics if the memory is already initialized at least once.
    #[inline]
    pub fn init(&mut self, multiboot_information_address: usize) {
        assert!(
            !self.is_mem_init.load(Ordering::Acquire),
            "The memory is initialized already."
        );

        *self = MMU::new_init(
            multiboot_information_address
        );
    }

    /// Maps a page to the frame at the same physical address as the page by a provided pointer
    /// within that page.
    /// 
    /// This is useful for directly mapping hardware tables or other critical structures
    /// that are expected to be at specific physical addresses.
    pub fn map_ptr<P>(&mut self, ptr: *const P, flags: EntryFlags) -> MMUResult {
        self.map_to(
            Page::containing_address(ptr as usize), 
            Frame::info_address(ptr as usize), 
            flags
        )
    }

    /// Maps the page to some free frame with the provided flags.
    ///
    /// This function is a wrapped interface that abstracts the need of providing frame allocator.
    pub fn map(&mut self, page: Page, flags: EntryFlags) -> MMUResult {
        self.with_active_table(|at, fa| at.map(page, flags, fa))
    }

    /// Maps the page to the frame with the provided flags.
    /// The `PRESENT` flag is added by default.
    ///
    /// This function is a wrapped interface that abstracts the need of providing frame allocator.
    pub fn map_to(&mut self, page: Page, frame: Frame, flags: EntryFlags) -> MMUResult {
        self.with_active_table(|at, fa| at.map_to(page, frame, flags, fa))
    }

    /// Unmaps the page that lies within the provided pointer.
    pub fn unmap_ptr<P>(&mut self, ptr: *const P) -> MMUResult {
        self.unmap(Page::containing_address(ptr as usize))
    }

    /// Unmaps the given page and adds all freed frames to the area frame allocator.
    ///
    /// This function is a wrapped interface that abstracts the need of providing frame allocator.
    pub fn unmap(&mut self, page: Page) -> MMUResult { 
        self.with_active_table(|at, fa| at.unmap(page, fa))
    }

    fn with_active_table<F>(&mut self, f: F) -> MMUResult 
        where F: FnOnce(&mut ActivePageTable, &mut AreaFrameAllocator)
    {
        if let Some(at) = &mut self.active_table {
            f(at, &mut self.frame_allocator);
            Ok(())
        } else {
            Err(MemError::NoFrameAlloc)
        }
    }

    /// Initiates and returns a new custom stack, with the current active page table.
    /// 
    /// You have to obtain the active page table, before you can. If it will be unable 
    /// to allocate guard page, starting page or the end page it will return None.
    ///
    /// # Panics
    /// 
    /// Panics if the main memory is not initialized.
    #[inline]
    pub fn allocate_stack(&mut self, size: usize) -> Option<Stack> {
        assert!(self.active_table.is_some(), "Cannot allocate stack, before the active page becomes available.");

        self.stack_allocator.alloc_stack(
            self.active_table.as_mut().unwrap(),
            &mut self.frame_allocator, 
            size
        )
    }

    /// Sets up a stack for interrupt stack.
    /// 
    /// This function sets the the stack for IST in the provided task state segment. Works
    /// only in Long Mode. The index is the entry index of the IST which must be a value
    /// in range 0..=6. Each index corresponds to IST entry, which is a custom stack for a 
    /// personal use to interrupt handle function. The size is the size value of the stack,
    /// where 1 is the size of one page.
    /// 
    /// # Warn
    /// 
    /// This should be done before loading the GDT table to the architecture. After setting the
    /// stack, the TSS must be loaded to the GDT in order to work properly.
    #[inline]
    pub fn set_interrupt_stack(&mut self, tss: &mut TSS, index: usize, size: usize) {
        assert!(index < 7, "The IST is only 7 entries long.");

        tss.interrupt_stack_pointers_table[index] = self
            .allocate_stack(size)
            .expect("Unable to allocate memory for IST.").top;
    }

    /// Sets up a stack for privilege stack table.
    /// 
    /// This function sets up the stack for PST in the provided task state segment. Works
    /// only in Long Mode. The index is the entry index of the PST which must be a value
    /// in range 0..=3. Each index corresponds to PST entry, which is a custom stack for
    /// privilege level switching. The size is the size value of the stack, where 1 is the 
    /// size of one page.
    /// 
    /// # Warn
    /// 
    /// This should be done before loading the GDT table to the architecture. After setting the
    /// stack, the TSS must be loaded to the GDT in order to work properly.
    #[inline]
    pub fn set_privilege_stack(&mut self, tss: &mut TSS, index: usize, size: usize) {
        assert!(index < 3, "The PST is only 3 entries long.");

        tss.privilege_stack_pointers_table[index] = self
            .allocate_stack(size)
            .expect("Unable to allocate memory for PST.").top;
    }

    /// Returns the page address in memory where some item is located.
    #[inline]
    pub fn address_of<T>(item: &T) -> usize {
        item as *const T as usize
    }

    /// Maps all ACPI tables during the initialization.
    ///
    /// Obtains the RSDT and parses all pointed tables to properly map all them based on their
    /// purpose and function. This function must be called only after full memory initialization.
    /// Otherwise a page fault will occur due to the access to different ACPI SDTs.
    #[inline]
    fn map_acpi_rsdt<A>(rsdt: RSDT, mapper: &mut InnerMapper, allocator: &mut A) where
        A: FrameAlloc
    {
        use crate::Color;
        use EntryFlags::*;

        for (mut h1, h2) in rsdt.pointers().iter().map(|(ptr)| unsafe {  
            let h1 = (ptr.v1[0] as *mut u32).cast::<ACPISDTHeader>(); 
            let h2 = (ptr.v1[1] as *mut u32).cast::<ACPISDTHeader>(); 
            (h1, h2)
        }) {
            // Performing twice due to non aligned 32-bit pointers.
            for _ in 0..2 {
                unsafe {
                    if !h1.is_null() {
                        println!("Mapping ACPI table: {:?}", h1.read_unaligned().signature);
                        let start = h1 as usize;
                        let start_frame = Frame::info_address(start);
                        let end_frame = Frame::info_address(start + h1.read_unaligned().length as usize);

                        for frame in Frame::range_inclusive(start_frame, end_frame) {
                            mapper.indentity_map(frame, WRITABLE, allocator);
                        }
                    }
                }
                h1 = h2;
            }
        }
    }

    /// Maps all ACPI tables during the initialization.
    ///
    /// Obtains the XSDT and parses all pointed tables to properly map all them based on their
    /// purpose and function. This function must be called only after full memory initialization.
    /// Otherwise a page fault will occur due to the access to different ACPI SDTs.
    #[inline]
    fn map_acpi_xsdt<A>(xsdt: XSDT, mapper: &mut InnerMapper, allocator: &mut A) where
        A: FrameAlloc
    {
        use crate::Color;
        use EntryFlags::*;

        for header in xsdt.pointers().iter().map(|(ptr)| unsafe {(ptr.v2 as *mut usize).cast::<ACPISDTHeader>()}) {
            unsafe {
                if !header.is_null() {
                    println!("Mapping ACPI table: {:?}", header.read_unaligned().signature);
                    let start = header as usize;
                    let start_frame = Frame::info_address(start);
                    let end_frame = Frame::info_address(start + header.read_unaligned().length as usize);

                    for frame in Frame::range_inclusive(start_frame, end_frame) {
                        mapper.indentity_map(frame, WRITABLE, allocator);
                    }
                }
            }
        }
    }

    /// Remaps sections of kernel.
    #[inline]
    fn remap_kernel<A>(allocator: &mut A, boot_info: &InfoPointer) -> ActivePageTable
        where A: FrameAlloc
    {
        use crate::kernel_components::arch_x86_64::acpi::rsdp::{RSDP, XSDP};
        use crate::Color;

        let mut temporary_page = TempPage::new( Page::containing_address(0xdeadbeef), allocator);
        let mut active_table = unsafe { ActivePageTable::new() };
        let mut new_table = {
            let frame = allocator.alloc().expect("no more frames to allocate.");
            InactivePageTable::new(frame, &mut active_table, &mut temporary_page)
        };

        active_table.with(&mut new_table, &mut temporary_page, |mapper| {
            use super::EntryFlags::{*, self};
            
            let elf_sections_tag = boot_info
                .elf_sections_tag()
                .expect("Elf-sections tag required.");

            for section in elf_sections_tag {
                use super::frames::PAGE_SIZE;

                if !section.is_allocated() {
                    continue
                }

                if section.start_address() % PAGE_SIZE as u64 != 0 {
                    panic!("Sections must be page aligned! 
                        Expected section to be {PAGE_SIZE} bytes aligned. 
                        Received value: {:#x} which is not properly aligned.\n
                        Section data:\n
                                name: {}, 
                                type: {:?}, 
                                addr: {:#x},
                                size: {:#x},
                                alignment constraints: {}, 
                                flags: {:#x}.\n", 
                            section.start_address(), 
                            section.name().unwrap_or("No name"),
                            section.section_type(),
                            section.start_address(), 
                            section.size(),
                            section.addralign(),
                            section.get().flags());
                }

                #[cfg(debug_assertions)] {
                    println!(Color::LIGHTGREEN; "Mapping section at addr: {:#x}, size: {:#x}", section.start_address(), section.size());
                }
                
                let flags = EntryFlags::from_elf_section_flags(&section);

                let start_frame = Frame::info_address(section.start_address() as usize);
                let end_frame = Frame::info_address(section.end_address() as usize - 1);
                
                for frame in Frame::range_inclusive(start_frame, end_frame) {
                    mapper.indentity_map(frame, flags, allocator);
                }
            }

            // identity map the multiboot info structure.
            let multiboot_start = Frame::info_address(boot_info.mstart());
            let multiboot_end = Frame::info_address(boot_info.mend());

            for frame in Frame::range_inclusive(multiboot_start, multiboot_end) {
                mapper.indentity_map(frame, PRESENT, allocator);
            }
            
            // identity map the VGA text buffer.
            let vga_buffer_frame = Frame::info_address(0xb8000);
            mapper.indentity_map(vga_buffer_frame, WRITABLE, allocator);

            #[cfg(debug_assertions)] {
                println!(Color::LIGHTGREEN; "Mapping ACPI tables.");
            }

            // identity map the XSDT/RSDT and the rest of ACPI tables.
            //
            // We would have to go one by one for each ACPI table. Because of that
            // RSDT is necessary to obtain all pointers to all tables and map them
            // individually.
            if let Some(x) = boot_info.get_tag::<ACPITagNew>() {
                    // Have to firstly map the table before actually using it.
                    let xsdt = unsafe { XSDT::from_xsdp(x.xsdp.clone()) };
                    let xsdt_start = Frame::info_address(x.xsdp.ptr as usize);
                    let xsdt_end = Frame::info_address(xsdt_start.num + xsdt.header.length as usize);

                    // Mapping the RSDT itself
                    for frame in Frame::range_inclusive(xsdt_start, xsdt_end) {
                        mapper.indentity_map(frame, PRESENT, allocator);
                    }

                    // Mapping each ACPI table.
                    MMU::map_acpi_xsdt(xsdt, mapper, allocator);
            } else {
                crate::warn!("XSDT is not present, mapping the legacy RSDT instead.");
                if let Some(r) = boot_info.get_tag::<ACPITagOld>() {
                    // Have to firstly map the table before actually using it.
                    let rsdt = unsafe { RSDT::from_rsdp(r.rsdp.clone()) };
                    let rsdt_start = Frame::info_address(r.rsdp.ptr as usize);
                    let rsdt_end = Frame::info_address((r.rsdp.ptr + rsdt.header.length) as usize);

                    // Mapping the RSDT itself
                    for frame in Frame::range_inclusive(rsdt_start, rsdt_end) {
                        mapper.indentity_map(frame, PRESENT, allocator);
                    }

                    // Mapping each ACPI table.
                    MMU::map_acpi_rsdt(rsdt, mapper, allocator);
                } else {
                    panic!("RSDT is not present. Unable to identity map.");
                }
            }
        });

        let old_table = active_table.switch(new_table);
        let old_p4_page = Page::containing_address(
            old_table.p4_frame.start_address()
        );

        active_table.unmap(old_p4_page, allocator);
        #[cfg(debug_assertions)] {
            println!(Color::LIGHTGRAY; "Guard page at {:#x}", old_p4_page.start_address());
        }

        active_table
    }

    /// Gets a tag of the rsdp pointer, which is copied by multiboot2 during boot process.
    pub(crate) fn get_rsdp(&self) -> Option<&ACPITagOld> {
        self.info_pointer.get_tag::<ACPITagOld>()
    }

    /// Gets a tag of the xsdp pointer, which is copied by multiboot2 during boot process.
    pub(crate) fn get_xsdp(&self) -> Option<&ACPITagNew> {
        self.info_pointer.get_tag::<ACPITagNew>()
    }
}

/// Enum that defines memory related errors.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MemError {
    NoFrameAlloc,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(C)]
pub struct BootInfoHeader {
    pub total: u32,
    _reserved: u32,
}

/// Boot information structure.
/// 
/// This structure provides the header and the tags related to boot info from the
/// GRUB's multiboot structure.
#[derive(Debug)]
#[repr(C)]
pub struct BootInfo {
    header: BootInfoHeader,
    tags: [u8],
}

impl BootInfoHeader {
    fn new(size: u32) -> Self {
        Self {
            total: size,
            _reserved: 0,
        }
    }
}

impl BootInfo {
    fn end_tag_check(&self) -> bool {
        let end_temp = EndTag::default();
        let self_ptr = unsafe { 
            self.tags
            .as_ptr()
            .sub(size_of::<BootInfoHeader>()) 
        };
        let end_tag_ptr = unsafe {
            self_ptr
            .add(self.header.total as usize)
            .sub(size_of::<EndTag>())    
        };
        let end_tag = unsafe { &*(end_tag_ptr as *const EndTag) };
        end_tag.tag_type == end_temp.tag_type && end_tag.size == end_temp.size
    }
}

// MBI pointer wrapper for boot information
#[repr(transparent)]
#[derive(Debug, Copy)]
pub struct InfoPointer<'a>(&'a BootInfo);

impl<'a> InfoPointer<'a> {
    // Loads the info from the pointer.
    pub unsafe fn load(ptr: *const BootInfoHeader) -> Result<Self, MbiLoadError> {
        if ptr.is_null() || ptr.align_offset(8) != 0 {
            return Err(MbiLoadError::IllegalAddress)
        }

        let mbi = &*ptr;

        if mbi.total == 0 || mbi.total & 0b111 != 0 {
            return Err(MbiLoadError::IllegalTotalSize(mbi.total))
        }

        let slice_size = mbi.total as usize - size_of::<BootInfoHeader>();
        let mbi = &*core::ptr::from_raw_parts::<BootInfo>(ptr.cast(), slice_size);
        
        if !mbi.end_tag_check() {
            return Err(MbiLoadError::NoEndTag);
        }

        Ok(Self(mbi))     
    }

    /// Returns memory map tag. This tag points to memory areas inside the kernel.
    pub fn memory_map_tag(&self) -> Option<&MemoryMapTag> {
        self.get_tag::<MemoryMapTag>()
    }
    
    pub fn elf_sections_tag(&self) -> Option<SectionIter> {
        let tag = self.get_tag::<SectionsTag>();
        tag.map(|t| {
            assert!((t.entry_size * t.shndx) <= t.size);
            t.sections()
        })
    }

    pub fn get_tag<TagT: TagTrait + ?Sized + 'a>(&'a self) -> Option<&'a TagT> {
        self.tags()
            .find(|tag| tag.tag_type == TagT::ID.into())
            .map(|tag| tag.cast_tag::<TagT>())
    }

    /// Returns the total size of boot info header.
    pub fn total(&self) -> u32 {
        self.0.header.total
    }

    /// Returns the start of the kernel
    pub fn kstart(&self) -> u64 {
        self
            .elf_sections_tag()
            .expect("Elf-sections tag required.")
            .map(|s| s.get().addr())
            .min()
            .unwrap()
    }

    /// Returns the end of the kernel.
    pub fn kend(&self) -> u64 {
        self
            .elf_sections_tag()
            .expect("Elf-sections tag required.")
            .map(|s| s.get().addr())
            .max()
            .unwrap()
    }

    /// Returns the start of multiboot structure.
    pub fn mstart(&self) -> usize {
        &self.0.header as *const BootInfoHeader as usize
    }

    /// Returns the end of multiboot structure.
    pub fn mend(&self) -> usize {
        self.mstart() + ( self.total() as usize )
    }
   
    /// Returns an old ACPI tag, which contains RSDP pointer of ACPI v1.0
    pub fn acpi_old(&self) -> Option<&ACPITagOld> {
        self.get_tag::<ACPITagOld>()
    }

    /// Returns a new ACPI tag, which caintains XSDP pinter of ACPI v2.0
    pub fn acpi_new(&self) -> Option<&ACPITagNew> {
        self.get_tag::<ACPITagNew>()
    }

    fn tags(&self) -> TagIter {
        TagIter::new(&self.0.tags)
    }
}

impl<'a> Clone for InfoPointer<'a> {
    fn clone(&self) -> Self {
        unsafe { 
            InfoPointer::load(
                self.mstart() as *const BootInfoHeader 
            ) 
        }.unwrap()
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MbiLoadError {
    IllegalAddress,
    IllegalTotalSize(u32),
    NoEndTag,
}

impl Display for MbiLoadError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::IllegalAddress => {
                write!(f, "Illegal address encountered during MBI load")
            }
            Self::IllegalTotalSize(size) => {
                write!(f, "Illegal total size encountered during MBI load: {}", size)
            }
            Self::NoEndTag => {
                write!(f, "No end tag found during MBI load")
            }
        }
    }
}

impl Error for MbiLoadError {}

#[test_case]
fn memory_areas_test() {
    use crate::{println, print, Color};

    let multiboot_memory_address = 475552;

    let boot_info = unsafe { InfoPointer::load(multiboot_memory_address as *const BootInfoHeader ) }.unwrap();
    let memory_map_tag = boot_info.memory_map_tag()
        .expect("Memory map tag required.");

    let kernel_start = boot_info.kstart();
    let kernel_end = boot_info.kend();
    let multiboot_start = boot_info.mstart();
    let multiboot_end = boot_info.mend();

    println!("Memory Areas:");
    for area in memory_map_tag.memory_areas() {
        println!(Color::GREEN; "      start: 0x{:x}, length: 0x{:x}", area.base_addr, area.length);
    }
}

#[test_case]
fn kernel_sections_test() {
    use crate::{println, print, Color};

    let multiboot_memory_address = 475552;

    let boot_info = unsafe { InfoPointer::load(multiboot_memory_address as *const BootInfoHeader ) }.unwrap();
    let memory_map_tag = boot_info.memory_map_tag()
        .expect("Memory map tag required.");
    let elf_sections_tag = boot_info.elf_sections_tag()
        .expect("Elf-sections tag required.");

    let kernel_start = boot_info.kstart();
    let kernel_end = boot_info.kend();
    let multiboot_start = boot_info.mstart();
    let multiboot_end = boot_info.mend();

    println!("Kernel Sections:");
    for (num, section) in elf_sections_tag.enumerate() {
        let section_inner = section.get();
        println!(Color::LIGHTGREEN; "      addr: 0x{:x}, size: 0x{:x}, flags: 0x{:x}, number: {}", section_inner.addr(), section_inner.size(), section_inner.flags(), num);
    }
}

#[test_case]
fn frame_allocator_test() {
    use crate::{println, print, Color};
    use super::{AreaFrameAllocator, frames::FrameAlloc};

    let multiboot_memory_address = 475552;

    let boot_info = unsafe { InfoPointer::load(multiboot_memory_address as *const BootInfoHeader ) }.unwrap();
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

    println!("Allocating all of the frames!");
    for i in 0.. {
        if let None = frame_allocator.alloc() {
            println!(Color::MAGENTA; "Allocated {} frames", i);
            break;
        }
    }
} 

#[test_case]
fn mapping_kernel() {
    use crate::{println, print, 
        kernel_components::
            memory::{
                InfoPointer, BootInfoHeader, 
                AreaFrameAllocator, ActivePageTable, Page,
                EntryFlags,
                frames::FrameAlloc  
        },            
        Color,
    };
    
    let multiboot_memory_address = 475552;
    let boot_info = unsafe { InfoPointer::load(multiboot_memory_address as *const BootInfoHeader ) }.unwrap();
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
    
    let mut page_table = unsafe { ActivePageTable::new() };
    
    let addr = 42 * 512 * 512 * 4096; // 42th P3 entry.
    let page = Page::containing_address(addr);
    let frame = frame_allocator.alloc().expect("No more frames to allocate.");
    
    println!(Color::CYAN; "None = {:?}, map to {:?}", page_table.translate(addr), frame);
    page_table.map_to(page, frame, EntryFlags::empty(), &mut frame_allocator);
    println!(Color::LIGHTBLUE; "Some = {:?}", page_table.translate(addr));
    println!(Color::LIGHTGREEN; "Next free frame: {:?}", frame_allocator.alloc());
    
    println!("{:#x}", unsafe {
        *(Page::containing_address(addr).start_address() as *const u64)
    });
    
    page_table.unmap(Page::containing_address(addr), &mut frame_allocator);
    println!(Color::MAGENTA; "None = {:?}", page_table.translate(addr));
}
