/// Inactive page tables give the possibility of using 'ActivePageTable's
/// methods on inactive pages, in order to remap inactive pages.

use super::{frames::Frame, temporary_pages::TempPage, ActivePageTable};

/// The main struct for inactive pages.
pub struct InactivePageTable {
    pub p4_frame: Frame,
}

impl InactivePageTable {
    pub fn new(frame: Frame, active_table: &mut ActivePageTable, temp_page: &mut TempPage) -> Self {
        use super::EntryFlags::*;
        {
            let table = temp_page.map_table_frame(frame.clone(), active_table);

            table.zero();
            // This sets up recursive mapping for the table.
            table[511].set(frame.clone(), PRESENT | WRITABLE);
        }
        temp_page.unmap(active_table);

        Self { p4_frame: frame }
    }

    pub fn get_clone(&self) -> Frame {
        self.p4_frame.clone()
    }
}