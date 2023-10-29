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
#[derive(Debug)]
#[repr(C)]
pub struct Stack {
    pub top: usize,
    pub bottom: usize,
}

impl Stack {
    fn new(top: usize, bottom: usize) -> Self {
        assert!(top > bottom, "The top is smaller than the bottom.");

        Self {
            top, bottom
        }
    }
}