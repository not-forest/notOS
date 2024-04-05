//! Module that defines os specific data structures.
//!
//! All data types within this structure are used only for better readibility
//! and debugging. They dont have specific purpose, and because of that, defined there.

use core::fmt::{Display, Debug};

/// Special os type for unsigned character type, which is compatible with 'u_char'
/// representation in standart C library.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct UChar(pub u8);

impl PartialEq<u8> for UChar {
    fn eq(&self, other: &u8) -> bool {
        self.0 == *other
    }
}

impl Debug for UChar {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result { 
        write!(f, "{}", self.0 as char)
    }
} 

impl Display for UChar {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        Debug::fmt(self, f)
    }
}
