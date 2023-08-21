/// Physical memory management. Frames and allocation.

use super::memory_map::{MemoryArea, MemoryAreaIter};

/// The size of each individual page chunk.
const PAGE_SIZE: usize = 4096;

/// A frame structure, which is just a pointer counter to the next frame
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Frame {
    num: usize,
}

impl Frame {
    fn info_address(address: usize) -> Frame {
        Frame { num: address / PAGE_SIZE }
    }
}

/// Frame allocator struct. The next_free_frame field is a simple counter that is increased every time we return a frame. 
/// It’s initialized to 0 and every frame below it counts as used. The current_area field holds the memory area that 
/// contains next_free_frame. If next_free_frame leaves this area, we will look for the next one in areas. 
/// When there are no areas left, all frames are used and current_area becomes None. The {kernel, multiboot}_{start, end} 
/// fields are used to avoid returning already used fields.
#[derive(Debug)]
pub struct AreaFrameAllocator {
    next_free_frame: Frame,
    pub current_area: Option<&'static MemoryArea>,
    pub areas: MemoryAreaIter,
    kernel_start: Frame,
    kernel_end: Frame,
    multiboot_start: Frame,
    multiboot_end: Frame,
}

impl AreaFrameAllocator {
    /// Creates a new frame allocator.
    pub fn new(
        kernel_start: usize,
        kernel_end: usize,
        multiboot_start: usize,
        multiboot_end: usize,
        memory_areas: MemoryAreaIter
    ) -> Self {
        let mut allocator = Self {
            next_free_frame: Frame::info_address(0),
            current_area: None,
            areas: memory_areas,
            kernel_start: Frame::info_address(kernel_start),
            kernel_end: Frame::info_address(kernel_end),
            multiboot_start: Frame::info_address(multiboot_start),
            multiboot_end: Frame::info_address(multiboot_end),
        };
        allocator.choose_next_area();
        allocator
    }

    /// Chooses the next free memory area to allocate a frame.
    fn choose_next_area(&mut self) {
        self.areas.next();
        self.current_area = self.areas.clone().filter(|area| {
            let address = area.base_addr + area.length - 1;
            Frame::info_address(address as usize) >= self.next_free_frame
        }).min_by_key(|area| area.base_addr);

        if let Some(area) = self.current_area {
            let start_frame = Frame::info_address(area.base_addr as usize);
            if self.next_free_frame < start_frame {
                self.next_free_frame = start_frame;
            }
        }
    }
}

/// Allocation trait that does the actual frame allocation.
pub trait FrameAlloc {
    fn alloc(&mut self) -> Option<Frame>;
    fn dealloc(&mut self, frame: Frame);
}

impl FrameAlloc for AreaFrameAllocator {
    /// Allocates the frame in the memory area. Returns the allocated Frame.
    fn alloc(&mut self) -> Option<Frame> {
        if let Some(area) = self.current_area {
            /// Fake "cloning". The Frame itself is not a Clone, therefore 
            /// we should construct the identical Frame.
            let frame = Frame { num: self.next_free_frame.num };

            let current_area_last_frame = {
                let address = area.base_addr + area.length - 1;
                Frame::info_address(address as usize)
            };

            if frame > current_area_last_frame {
                self.choose_next_area();
            } else if frame >= self.kernel_start && frame <= self.kernel_end {
                // `frame` is used by the kernel
                self.next_free_frame = Frame {
                    num: self.kernel_end.num + 1
                };
            } else if frame >= self.multiboot_start && frame <= self.multiboot_end {
                // `frame` is used by the multiboot information structure
                self.next_free_frame = Frame {
                    num: self.multiboot_end.num + 1
                };
            } else {
                self.next_free_frame.num += 1;
                return Some(frame);
            }

            self.alloc()
        } else {
            None
        }
    }

    fn dealloc(&mut self, frame: Frame) {
        unimplemented!()
    }
}



