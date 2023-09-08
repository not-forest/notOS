/// Temporary mapping allows us to map the frame to some
/// virtual address. 

use super::{
    Page, ActivePageTable, 
    frames::{FrameAlloc, Frame},
    paging::{Table, Level1},
};
use crate::VirtualAddress;

/// The main struct for temporary paging.
pub struct TempPage {
    page: Page,
    tiny_alloc: TinyAlloc,
}

impl TempPage {
    /// Creates a new temporary page, based on given page and allocator.
    pub fn new<A>(page: Page, allocator: &mut A) -> Self 
        where A: FrameAlloc
    {
        Self { 
            page, 
            tiny_alloc:  TinyAlloc::new(allocator)
        }
    }

    /// Maps the temporary page to the given frame in the active table.
    /// Returns the start address of the temporary page.
    pub fn map(&mut self, frame: Frame, active_table: &mut ActivePageTable) -> VirtualAddress {
        use super::EntryFlags::WRITABLE;

        assert!(
            active_table.translate_page(self.page).is_none(),
            "temporary page is already mapped!"
        );
        active_table.map_to(self.page, frame, WRITABLE, &mut self.tiny_alloc);
        self.page.start_address()
    }

    /// Maps the temporary page to the given page table frame in the active
    /// table. Returns a reference to the now mapped table. The unsafe block 
    /// is safe since the 'VirtualAddress' returned by the map function is 
    /// always valid and the type cast just reinterprets the frame’s content.
    pub fn map_table_frame(&mut self, frame: Frame, active_table: &mut ActivePageTable) -> &mut Table<Level1> {
        unsafe {
            &mut *(self.map(frame, active_table) as *mut Table<Level1>)
        }
    }

    /// Unmaps the temporary page in the active table.
    pub fn unmap(&mut self, active_table: &mut ActivePageTable) {
        active_table.unmap(self.page, &mut self.tiny_alloc)
    }
}

/// A special small allocator for temporary pages.
struct TinyAlloc([Option<Frame>; 3]);

impl FrameAlloc for TinyAlloc {
    fn alloc(&mut self) -> Option<Frame> {
        for frame_option in &mut self.0 {
            if frame_option.is_some() {
                return frame_option.take();
            }
        }
        None
    }

    fn dealloc(&mut self, frame: Frame) {
        for frame_option in &mut self.0 {
            if frame_option.is_none() {
                *frame_option = Some(frame);
                return;
            }
        }
        panic!("Tiny allocator can only hold up to 3 frames.")
    }
}

impl TinyAlloc {
    /// Creates a new instance of 'TinyAlloc' and allocating three
    /// available slots.
    fn new<A>(allocator: &mut A) -> Self 
        where A: FrameAlloc
    {
        let mut cl = || allocator.alloc();
        let frames = [cl(), cl(), cl()];
        Self(frames)
    }
}
