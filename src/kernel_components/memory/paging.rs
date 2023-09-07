
/// Paging memory management scheme module. It separates the physical frames
/// with it's virtual pages.

use super::frames::{Frame, FrameAlloc, PAGE_SIZE};
use crate::{
    PhysicalAddress, VirtualAddress,
    kernel_components::structures::IternumTrait,
    bitflags,
};
use core::ops::{Index, IndexMut};
use core::marker::PhantomData;

/// A total amount of entries.
pub(crate) const ENTRY_COUNT: usize = 512;
/// A mask that masks bits 12-51.
pub const BIT_MASK: usize = 0x000fffff_fffff000;

/// The address of P4 table in the kernel.
pub const P4: *mut Table<Level4> = 0o177777_777_777_777_777_0000 as *mut _;

/// Table levels. The use of type system to guarantee that the next_table methods
/// can only be called on P4, P3 and P2 tables.

/// Table level trait is just a marker that satisfies compiler's trait bounds about table levels,
/// by saying that this table level is in fact a table level and nothing else. It is an empty market trait.
pub trait TableLevel {}
/// Hierarchical level trait is used to give info about next table level, if it does exist.
pub trait HierarchicalLevel: TableLevel {
    type NextLevel: TableLevel;
}

pub enum Level4 {}
pub enum Level3 {}
pub enum Level2 {}
pub enum Level1 {}

impl TableLevel for Level4 {}
impl TableLevel for Level3 {}
impl TableLevel for Level2 {}
impl TableLevel for Level1 {}

impl HierarchicalLevel for Level4 {
    type NextLevel = Level3;
}

impl HierarchicalLevel for Level3 {
    type NextLevel = Level2;
}

impl HierarchicalLevel for Level2 {
    type NextLevel = Level1;
}

/// A representation of a single page. It is just like a Frame, but virtual.
#[derive(Debug, Clone, Copy)]
pub struct Page {
    num: usize
}

impl Page {
    /// Checks virtual address bounds and returns the page.
    pub fn containing_address(address: VirtualAddress) -> Self {
        assert!(
            address < 0x0000_8000_0000_0000 || address >= 0xffff_8000_0000_0000,
            "Invalid address: 0x{:x}", address
        );
        Self { num: address / PAGE_SIZE }
    }

    /// Returns the starting address of the page.
    pub fn start_address(&self) -> usize {
        self.num * PAGE_SIZE
    }

    pub(crate) fn p4_index(&self) -> usize {
        (self.num >> 27) & 0o777
    }
    pub(crate) fn p3_index(&self) -> usize {
        (self.num >> 18) & 0o777
    }
    pub(crate) fn p2_index(&self) -> usize {
        (self.num >> 9) & 0o777
    }
    pub(crate) fn p1_index(&self) -> usize {
        (self.num >> 0) & 0o777
    }
}

/// A representation of a single entry.
pub struct Entry(u64);

impl Entry {
    pub fn flags(&self) -> u64 {
        EntryFlags::from_bits_truncate(self.0).into()
    }

    pub fn pointed_frame(&self) -> Option<Frame> {
        if EntryFlags::PRESENT.is_in(self.flags()) {
            Some(Frame::info_address(
                self.0 as usize & BIT_MASK
            ))
        } else {
            None
        }
    }

    pub fn set(&mut self, frame: Frame, flags: EntryFlags) {
        assert!(frame.start_address() & !BIT_MASK == 0);
        self.0 = (frame.start_address() as u64) | u64::from(flags);
    }

    pub fn is_unused(&self) -> bool {
        self.0 == 0
    }

    pub fn set_unused(&mut self) {
        self.0 = 0;
    }
}

/// The page table itself. It contains all the entries.
pub struct Table<L: TableLevel> {
    entries: [Entry; ENTRY_COUNT],
    level: PhantomData<L>,
}

impl<L: TableLevel> Table<L> {
    pub fn zero(&mut self) {
        for entry in self.entries.iter_mut() {
            entry.set_unused();
        }
    }
}

impl<L: HierarchicalLevel> Table<L> {
    pub fn next_table(&self, index: usize) -> Option<&Table<L::NextLevel>> {
        self.next_table_address(index)
            .map(|address| unsafe { &*(address as *const _) })
    }

    pub fn next_table_mut(&mut self, index: usize) -> Option<&mut Table<L::NextLevel>> {
        self.next_table_address(index)
            .map(|address| unsafe { &mut *(address as *mut _) })
    }

    pub fn next_table_create<A>(&mut self, index: usize, allocator: &mut A) -> &mut Table<L::NextLevel>
        where A: FrameAlloc
    {
        use EntryFlags::*;
        if self.next_table(index).is_none() {
            assert!(!HUGE_PAGE.is_in(self.entries[index].flags()), "Mapping code does not support huge pages.");

            let frame = allocator.alloc().expect("No frames available.");
            self.entries[index].set(frame, PRESENT | WRITABLE);
            self.next_table_mut(index).unwrap().zero();
        }
        self.next_table_mut(index).unwrap()
    }

    fn next_table_address(&self, index: usize) -> Option<usize> {
        use EntryFlags::*;

        let entry_flags = self[index].flags();
        if PRESENT.is_in(entry_flags) && !HUGE_PAGE.is_in(entry_flags) {
            let table_address = self as *const _ as usize;
            Some((table_address << 9) | (index << 12))
        } else {
            None
        }
    }
}

impl<L: TableLevel> Index<usize> for Table<L> {
    type Output = Entry;

    fn index(&self, index: usize) -> &Self::Output {
        &self.entries[index]
    }
}

impl<L: TableLevel> IndexMut<usize> for Table<L> {
    fn index_mut(&mut self, index: usize) -> &mut Entry {
        &mut self.entries[index]
    }
}

/// Bitflags that hold information about the physical address.
bitflags! {
    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub struct EntryFlags: u64 {
        /// The page is curently in memory.
        const PRESENT =         1 << 0,
        /// It is allowed to write to that page
        const WRITABLE =        1 << 1,
        /// Tells that user must not enter the kernel mode to access this page.
        const USER_ACCESSIBLE = 1 << 2,
        /// Writes go directly to memory through caching.
        const WRITE_THROUGH =   1 << 3,
        /// No cache is used for this page.
        const NO_CACHE =        1 << 4,
        /// The CPU sets this bit when this page is being used.
        const ACCESSED =        1 << 5,
        /// The CPU sets this bit when a write to page occurs.
        const DIRTY =           1 << 6,
        /// This must be 0 in P1 and P4, creates a 1GiB page in P3,
        /// creates a 2MiB page in P2.
        const HUGE_PAGE =       1 << 7,
        /// The 	page isn’t flushed from caches on address space
        /// switch (PGE bit of CR4 register must be set).
        const GLOBAL =          1 << 8,
        /// Forbid executing code on this page (the NXE bit in the
        /// EFER register must be set).
        const NO_EXECUTE =      1 << 63,
    }
}

#[test_case]
#[should_panic]
fn test_pages() {
    let p4 = unsafe { &*P4 };
    p4.next_table(42)
      .and_then(|p3| p3.next_table(1337))
      .and_then(|p2| p2.next_table(0xdeadbeaf));
}