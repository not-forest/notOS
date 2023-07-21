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

impl Error for PoisonError {}