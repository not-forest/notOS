/// Ownership for the P4 table in paging module. It is a safe wrapper around
/// Table<Level4> struct. The safety is required to prevent data race: For example, 
/// imagine that one thread maps an entry to frame_A and another thread (on the same core) 
/// tries to map the same entry to frame_B. The problem is that there’s no 
/// clear owner for the page tables.

use super::{
    paging::{Table, Page, Level4, EntryFlags, ENTRY_COUNT, P4},
    frames::{Frame, FrameAlloc, PAGE_SIZE}, 
    inactive_tables::InactivePageTable, 
    temporary_pages::TempPage,
};
use crate::{VirtualAddress, PhysicalAddress, println};
use crate::kernel_components::arch_x86_64::TLB;
use core::ptr::NonNull;
use core::ops::{Deref, DerefMut};

/// This struct is a wrapper over the mapper struct. 
pub struct ActivePageTable {
    inner: InnerMapper
}

impl ActivePageTable {
    /// Creates a new instance of 'ActivePageTable'. This function is
    /// safe, as long, as the pointer to P4 from the boot.asm is right.
    pub unsafe fn new() -> Self {
        Self { inner: InnerMapper::new() }
    }

    /// Temporary changes the recursive mapping and executes a given closure in the new context.
    /// 
    /// It overwrites the 511th P4 entry and points it to the inactive table frame. 
    /// Then it flushes the translation lookaside buffer, which still contains some 
    /// old translations.
    pub fn with<F>(&mut self, table: &mut InactivePageTable, temp_page: &mut TempPage, f: F) 
        where F: FnOnce(&mut InnerMapper)
    {
        use crate::kernel_components::registers::control::Cr3;
        use EntryFlags::*;

        {
            // Get the backup table and map temporary page to current p4 table.
            let (backup_frame, _) = Cr3::read();
            let p4_table = temp_page.map_table_frame(backup_frame.clone(), self);

            // overwrite recursive mapping.
            self.get_mut()[511].set(table.get_clone(), PRESENT | WRITABLE);
            TLB::flush_all();

            // execute f in the new context.
            f(self);

            // restore recursive mapping to original p4 table.
            p4_table[511].set(backup_frame, PRESENT | WRITABLE);
            TLB::flush_all();
        }

        temp_page.unmap(self);
    }

    /// Switches the current active table and returns the previous active
    /// table as 'InactivePageTable'.
    pub fn switch(&mut self, new_table: InactivePageTable) -> InactivePageTable {
        use crate::kernel_components::registers::control::Cr3;
        let new_frame = Frame::info_address(new_table.p4_frame.start_address());
        let (frame, flags) = Cr3::read();

        let old_table = InactivePageTable {
            p4_frame: frame
        };

        unsafe {
            Cr3::write(new_frame, flags);
        }
        old_table
    }

    fn get(&self) -> &Table<Level4> {
        unsafe { self.p4.as_ref() }
    }

    fn get_mut(&mut self) -> &mut Table<Level4> {
        unsafe { self.p4.as_mut() }
    }
}

impl Deref for ActivePageTable {
    type Target = InnerMapper;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for ActivePageTable {
    fn deref_mut(&mut self) -> &mut InnerMapper {
        &mut self.inner
    }
}

/// This struct is a wrapper around the active P4 table that owns
/// the ownership of the table itself.
pub struct InnerMapper {
    p4: NonNull<Table<Level4>>
}

impl InnerMapper {
    /// Creates a new instance of 'Mapper'. This function is
    /// safe, as long, as the pointer to P4 from the boot.asm is right.
    pub unsafe fn new() -> Self {
        Self { p4: NonNull::new(P4).unwrap() }
    }
    
    /// Translates a virtual to the corresponding physical address.
    /// Returns `None` if the address is not mapped.
    pub fn translate(&self, address: VirtualAddress) -> Option<PhysicalAddress> {
        let offset = address % PAGE_SIZE;
        self.translate_page(Page::containing_address(address))
            .map(|frame| frame.num * PAGE_SIZE + offset)
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
    
    /// Maps the page to the frame with the provided flags.
    /// The `PRESENT` flag is added by default. Needs a
    /// `FrameAllocator` as it might need to create new page tables.
    pub fn map_to<A>(&mut self, page: Page, frame: Frame, flags: EntryFlags, allocator: &mut A) 
        where A: FrameAlloc 
        {
            use EntryFlags::*;
            
        let p4 = self.get_mut();
        let mut p3 = p4.next_table_create(page.p4_index(), allocator);
        let mut p2 = p3.next_table_create(page.p3_index(), allocator);
        let mut p1 = p2.next_table_create(page.p2_index(), allocator);
        
        if !p1[page.p1_index()].is_unused() {
            crate::warn!("The page must be unused.\nReceived page {:?} with address: {:#x} that is currently used.", page, page.start_address());
        }
        
        p1[page.p1_index()].set(frame, flags | PRESENT);
    }

    /// Maps the page to some free frame with the provided flags.
    /// The free frame is allocated from the given `FrameAllocator`.
    pub fn map<A>(&mut self, page: Page, flags: EntryFlags, allocator: &mut A)
        where A: FrameAlloc
        {
        let frame = allocator.alloc().expect("Out of memory.");
        self.map_to(page, frame, flags, allocator)
    }

    /// Identity map the the given frame with the provided flags.
    /// The `FrameAllocator` is used to create new page tables if needed.
    pub fn indentity_map<A>(&mut self, frame: Frame, flags: EntryFlags, allocator: &mut A)
    where A: FrameAlloc
    {
        let page = Page::containing_address(frame.start_address());
        self.map_to(page, frame, flags, allocator)
    }

    /// Unmaps the given page and adds all freed frames to the given
    /// `FrameAllocator`.
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
        
        // Flushing the given address in the TLB via 'invlpg' asm instruction. 
        TLB::flush(page.start_address());
        
        // TODO free p(1,2,3) table if empty
        // allocator.dealloc(frame);
    }

    fn get(&self) -> &Table<Level4> {
        unsafe { self.p4.as_ref() }
    }

    fn get_mut(&mut self) -> &mut Table<Level4> {
        unsafe { self.p4.as_mut() }
    }
}

impl Deref for InnerMapper {
    type Target = Table<Level4>;
    fn deref(&self) -> &Self::Target {
        self.get()
    }
}

impl DerefMut for InnerMapper {
    fn deref_mut(&mut self) -> &mut Table<Level4> {
        self.get_mut()
    }
}

