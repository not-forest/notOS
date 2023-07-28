// Custom errors

use core::fmt::{Debug, Display};
use core::error::Error;

#[derive(Debug)]
pub struct PoisonError;

impl Display for PoisonError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "The mutex is poisoned and cannot longer be used")
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MbiLoadError {
    IllegalAddress,
    IllegalTotalSize(u32),
    NoEndTag,
}

impl Display for MbiLoadError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::IllegalAddress => {
                write!(f, "Illegal address encountered during MBI load")
            }
            Self::IllegalTotalSize(size) => {
                write!(f, "Illegal total size encountered during MBI load: {}", size)
            }
            Self::NoEndTag => {
                write!(f, "No end tag found during MBI load")
            }
        }
    }
}

impl Error for MbiLoadError {}
impl Error for PoisonError {}