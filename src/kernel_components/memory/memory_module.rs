// Memory module for memory management. This is the entry point of memory functions and structs. 

use core::mem::size_of;
use core::fmt::{Debug, Display};
use core::error::Error;
use crate::{VirtualAddress, PhysicalAddress, println};

use super::{
    Page, ActivePageTable,
    tags::{EndTag, TagTrait, TagIter}, 
    memory_map::MemoryMapTag,
    sections::{SectionsTag, SectionIter}, 
    frames::{Frame, FrameAlloc}, 
    temporary_pages::TempPage, 
    inactive_tables::InactivePageTable,
};

// This magic number has 
pub const MAGIC: u32 = 0x36d76289;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(C)]
pub struct BootInfoHeader {
    pub total: u32,
    _reserved: u32,
}

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
    
    fn tags(&self) -> TagIter {
        TagIter::new(&self.0.tags)
    }
}

/// Returns the page address in memory where some item is located.
pub fn address_of<T>(item: &T) -> usize {
    item as *const T as usize
}

/// Remaps sections of kernel.
pub fn remap_kernel<A>(allocator: &mut A, boot_info: &InfoPointer) -> ActivePageTable
    where A: FrameAlloc
{
    use crate::Color;

    let mut temporary_page = TempPage::new( Page::containing_address(0xdeadbeaf), allocator);
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

/// Creates a new frame allocator based on kernel multiboot start/end, remaps the kernel and
/// creates a guard page. Initiates a stack and a heap.
/// 
/// # Note
/// 
/// This function only does memory based operations without changing states of any register or
/// global state. Any specific properties of registers related to memory must be set manually
/// via kernel_components::registers::control.
pub fn init(boot_info: &InfoPointer) {
    use crate::{
        println,
        kernel_components::{
            memory::{
                InfoPointer, BootInfoHeader, AreaFrameAllocator, EntryFlags,
                allocators::GLOBAL_ALLOCATOR,
                paging::Page, self
            },
            registers::control,
        },            
        Color,
    };

    let memory_map_tag = boot_info.memory_map_tag()
        .expect("Memory map tag required.");

    let kernel_start = boot_info.kstart();
    let kernel_end = boot_info.kend();
    let multiboot_start = boot_info.mstart();
    let multiboot_end = boot_info.mend();
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
    let mut active_table = memory::remap_kernel(&mut frame_allocator, &boot_info);
    #[cfg(debug_assertions)] { println!("Remapping complete!"); }

    let heap_start_page = Page::containing_address(heap_start);
    let heap_end_page = Page::containing_address(heap_end);

    #[cfg(debug_assertions)] { println!("Mapping the heap pages."); }

    for page in Page::range_inclusive(heap_start_page, heap_end_page) {
        active_table.map(page, EntryFlags::WRITABLE, &mut frame_allocator);
        #[cfg(debug_assertions)]
        println!(Color::LIGHTGRAY; "Mapping page at address {:#x}", page.start_address());
    }   

    #[cfg(debug_assertions)] { println!("Mapping complete."); }
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