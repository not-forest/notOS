/// A module for allocating special kind of stack for personal use.
/// 
/// It could be useful for TSS segment, that contain privilege level stack table 
/// and interrupt stack table. Personal tables for software use can be also
/// allocated via this module.
 
use super::frames::{PAGE_SIZE, FrameAlloc};
use super::paging::{self, Page, PageIter};
use super::owned_tables::ActivePageTable;
use super::EntryFlags::WRITABLE;
use crate::{println, Color};

/// An allocators struct.
/// 
/// Stack allocator could be useful for TSS segment, that contain privilege level 
/// stack table and interrupt stack table. Personal tables for software use can be 
/// also allocated via this struct.
#[derive(Debug)]
pub struct StackAlloc {
    next_page: Page,
}

impl StackAlloc {
    /// Creates a new stack allocator.
    /// 
    /// The range must be an iterator over the pages, that you wish to allocate
    /// for a custom stack. The address must be unused unmapped memory location.
    pub fn new(page: Page) -> Self {
        Self { next_page: page }
    }

    /// Allocates the stack and returns it.
    /// 
    /// If it will be unable to allocate guard page, starting page or the end page,
    /// it will return None.
    pub fn alloc_stack<A>(
        &mut self,
        active_table: &mut ActivePageTable,
        frame_allocator: &mut A,
        size: usize
    ) -> Option<Stack> where A: FrameAlloc {
        if size == 0 {
            return None
        }
        
        let mut range = {
            let start = self.next_page;
            let end = self.next_page + size;

            Page::range_inclusive(start, end)
        };

        if let Some(_) = range.next() {
            let stack_start = range.next();
            let stack_end = if size == 1 {
                stack_start
            } else {
                range.nth(size - 2)
            };

            match (stack_start, stack_end) {
                (Some(start), Some(end)) => {
                    self.next_page = end;

                    for page in Page::range_inclusive(start, end) {
                        active_table.map(page, WRITABLE, frame_allocator);

                        #[cfg(debug_assertions)]
                        println!(Color::LIGHTGRAY; "Mapping stack page at address {:#x}", page.start_address());
                    }

                    Some(Stack::new(
                        end.start_address() + PAGE_SIZE,
                        start.start_address(),
                    ))
                }
                _ => None,
            }
        } else {
            None
        }
    }
}

/// A struct representing the stack.
/// 
/// This struct is returned in stack allocator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(C)]
pub struct Stack {
    pub top: usize,
    pub bottom: usize,
}

impl Stack {
    /// Creates a new instance of stack.
    ///
    /// The top must be bigger thatn the bottom in order for stack to make any sense.
    #[inline]
    pub(crate) fn new(top: usize, bottom: usize) -> Self {
        assert!(top > bottom, "The top is smaller than the bottom.");

        Self {
            top, bottom
        }
    }

    /// Retuns a total size of a stack.
    #[inline(always)]
    pub fn size(&self) -> usize {
        self.top - self.bottom
    }

    /// Checks if a pointer is within the stack and returns true if it is.
    #[inline(always)]
    pub fn contain(&self, ptr: usize) -> bool {
        ptr >= self.bottom && ptr <= self.top 
    }

    /// Shrinks the stack based on the input number.
    ///
    /// # Returns
    ///
    /// Returns the previous stack bottom inside the Ok(usize) if the stack shrinking was done
    /// successfully. Will return an Err(usize) if not.
    ///
    /// # Note
    ///
    /// Here the amount is assumed to be in bytes.
    #[inline(always)]
    pub fn shrink(&mut self, amount: usize) -> Result<usize, usize> {
        let b = self.bottom;

        if self.size() > 0 {
            self.bottom -= amount;
            Ok(b)
        } else {
            Err(b)
        }
    }

    /// Shrinks or grows the stack to the provided size
    ///
    /// # Note
    ///
    /// Here the amount is assumed to be in bytes.
    ///
    /// # Panics
    ///
    /// This function will panic if the size provided is zero.
    #[inline(always)]
    pub fn resize_to(&mut self, size: usize) {
        assert!(size != 0, "The stack size could not be zero. Deallocate the stack instead.");

        self.bottom = self.top - size;
    }

    /// Grows the stack based on the input number.
    ///
    /// # Returns
    ///
    /// Returns a previous stack bottom.
    ///
    /// # Note
    ///
    /// Here the amount is assumed to be in bytes. 
    #[inline(always)]
    pub fn grow(&mut self, amount: usize) -> usize {
        let b = self.bottom;
        self.bottom += amount;
        b
    }

    /// Shift the stack in memory based on the given offset to the left side. That means closer to
    /// the smaller addresses.
    ///
    /// # Note
    ///
    /// Here the offset is assumed to be in bytes. This function will not actually move any data in
    /// memory, it is only changing inner number values so it could be used in some high level
    /// structures like MMU.
    #[inline(always)]
    pub fn shift_left(&mut self, offset: usize) {
        self.bottom -= offset;
        self.top -= offset;
    }

    /// Shift the stack in memory based on the given offset to the right side. That means closer to
    /// the bigger addresses.
    ///
    /// # Note
    ///
    /// Here the offset is assumed to be in bytes. This function will not actually move any data in
    /// memory, it is only changing inner number values so it could be used in some high level
    /// structures like MMU.
    #[inline(always)]
    pub fn shift_right(&mut self, offset: usize) {
        self.bottom += offset;
        self.top += offset;
    }
}
