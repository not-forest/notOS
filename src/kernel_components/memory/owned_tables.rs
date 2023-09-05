/// Ownership for the P4 table in paging module. It is a safe wrapper around
/// Table<Level4> struct. The safety is required to prevent data race: For example, 
/// imagine that one thread maps an entry to frame_A and another thread (on the same core) 
/// tries to map the same entry to frame_B. The problem is that there’s no 
/// clear owner for the page tables.

use super::{
    paging::{Table, Page, Level4, EntryFlags, ENTRY_COUNT, P4},
    frames::{Frame, FrameAlloc, PAGE_SIZE},
};
use crate::{VirtualAddress, PhysicalAddress};
use core::ptr::NonNull;
use core::ops::{Deref, DerefMut};
use core::arch::asm;

pub struct ActivePageTable {
    p4: NonNull<Table<Level4>>
}

impl ActivePageTable {
    pub unsafe fn new() -> Self {
        Self { p4: NonNull::new(P4).unwrap() }
    }

    /// Translates the page into the frame
    pub fn translate_page(&self, page: Page) -> Option<Frame> {
        use EntryFlags::*;
        let p3 = self.get().next_table(page.p4_index());
        
        let huge_page = || {
            p3.and_then(|p3| {
                let p3_entry = &p3[page.p3_index()];

                if let Some(start_frame) = p3_entry.pointed_frame() {
                    if HUGE_PAGE.is_in(p3_entry.flags()) {
                        assert!(start_frame.num % (ENTRY_COUNT * ENTRY_COUNT) == 0);
                        return Some(Frame {
                            num: start_frame.num + page.p2_index() * ENTRY_COUNT + page.p1_index()
                        });
                    }
                }
                if let Some(p2) = p3.next_table(page.p3_index()) {
                    let p2_entry = &p2[page.p2_index()];

                    if let Some(start_frame) = p2_entry.pointed_frame() {
                        if HUGE_PAGE.is_in(p2_entry.flags()) {
                            // address must be 2MiB aligned
                            assert!(start_frame.num % ENTRY_COUNT == 0);
                            return Some(Frame {
                                num: start_frame.num + page.p1_index()
                            });
                        }
                    }
                }
                None
            })
        };

        p3.and_then(|p3| p3.next_table(page.p3_index()))
        .and_then(|p2| p2.next_table(page.p2_index()))
        .and_then(|p1| p1[page.p1_index()].pointed_frame())
        .or_else(huge_page)
    }

    pub fn translate(&self, address: VirtualAddress) -> Option<PhysicalAddress> {
        let offset = address % PAGE_SIZE;
        self.translate_page(Page::containing_address(address))
            .map(|frame| frame.num * PAGE_SIZE + offset)
    }
    
    pub fn map_to<A>(&mut self, page: Page, frame: Frame, flags: EntryFlags, allocator: &mut A) 
        where A: FrameAlloc 
    {
        use EntryFlags::*;
    
        let p4 = self.get_mut();
        let mut p3 = p4.next_table_create(page.p4_index(), allocator);
        let mut p2 = p3.next_table_create(page.p3_index(), allocator);
        let mut p1 = p2.next_table_create(page.p2_index(), allocator);

        assert!(p1[page.p1_index()].is_unused());
        p1[page.p1_index()].set(frame, flags | PRESENT);
    }

    pub fn map<A>(&mut self, page: Page, flags: EntryFlags, allocator: &mut A)
        where A: FrameAlloc
    {
        let frame = allocator.alloc().expect("Out of memory.");
        self.map_to(page, frame, flags, allocator)
    }

    pub fn indentity_map<A>(&mut self, frame: Frame, flags: EntryFlags, allocator: &mut A)
        where A: FrameAlloc
    {
        let page = Page::containing_address(frame.start_address());
        self.map_to(page, frame, flags, allocator)
    }

    pub fn unmap<A>(&mut self, page: Page, allocator: &mut A)
        where A: FrameAlloc
    {
        assert!(self.translate(page.start_address()).is_some());

        let p1 = self.get_mut()
            .next_table_mut(page.p4_index())
            .and_then(|p3| p3.next_table_mut(page.p3_index()))
            .and_then(|p2| p2.next_table_mut(page.p2_index()))
            .expect("mapping code does not support huge areas.");

        
        let frame = p1[page.p1_index()].pointed_frame().unwrap();
        p1[page.p1_index()].set_unused();
        
        //Flushing the given address in the TLB via 'invlpg' asm instruction. 
        unsafe {
            asm!("invlpg [{}]", in(reg) page.start_address(), options(nostack, preserves_flags));
        }

        // TODO free p(1,2,3) table if empty
        //allocator.dealloc(frame);
    }

    fn get(&self) -> &Table<Level4> {
        unsafe { self.p4.as_ref() }
    }

    fn get_mut(&mut self) -> &mut Table<Level4> {
        unsafe { self.p4.as_mut() }
    }
}

impl Deref for ActivePageTable {
    type Target = Table<Level4>;
    fn deref(&self) -> &Self::Target {
        self.get()
    }
}

impl DerefMut for ActivePageTable {
    fn deref_mut(&mut self) -> &mut Table<Level4> {
        self.get_mut()
    }
}

