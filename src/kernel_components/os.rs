//! Module that defines os specific data structures.
//!
//! All data types within this structure are used only for better readibility
//! and debugging. They dont have specific purpose, and because of that, defined there.

use core::{fmt::{Debug, Display}, mem::MaybeUninit};
use super::drivers::keyboards::KeyCode;

/// Special structure for representing currenty pressed keyboard key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct OSChar {
    pub chr: Option<char>,
    pub key: KeyCode
}

impl OSChar {
    /// Initializes a new OSChar structure.
    pub const fn new(chr: Option<char>, key: KeyCode) -> Self {
        Self { chr, key }
    }

    /// Initializes zeroed OSChar for buffer filling.
    pub const fn zeroed() -> Self {
        Self { 
            chr: None,
            key: unsafe { MaybeUninit::zeroed().assume_init() },
        }
    }
}

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
