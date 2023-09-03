// Memory module for memory management. This is the entry point of memory functions and structs. 

use core::mem::size_of;
use crate::{ MbiLoadError, VirtualAddress, PhysicalAddress};

use super::{
    tags::{EndTag, TagTrait, TagIter}, 
    memory_map::MemoryMapTag,
    sections::{SectionsTag, SectionIter},
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
    
    fn tags(&self) -> TagIter {
        TagIter::new(&self.0.tags)
    }
}

#[test_case]
fn memory_areas_test() {
    use crate::{println, print, Color};

    let multiboot_memory_address = 475552;

    let boot_info = unsafe { InfoPointer::load(multiboot_memory_address as *const BootInfoHeader ) }.unwrap();
    let memory_map_tag = boot_info.memory_map_tag()
        .expect("Memory map tag required.");

    let kernel_start = boot_info.kstart();
    let kernel_end = boot_info.kend();
    let multiboot_start = multiboot_memory_address;
    let multiboot_end = multiboot_start + ( boot_info.total() as usize );

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
    let multiboot_start = multiboot_memory_address;
    let multiboot_end = multiboot_start + ( boot_info.total() as usize );

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
    let multiboot_start = multiboot_memory_address;
    let multiboot_end = multiboot_start + ( boot_info.total() as usize );

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