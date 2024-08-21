use alloc::boxed::Box;

/// A module that defines a global interface to OS clock.

use crate::kernel_components::drivers::Driver;

/// A clock driver trait.
///
/// All clock drivers must implement this trait for global in throughout the OS.
pub trait ClockDriver {
    /// Returns current date and time at that specific point.
    ///
    /// This function must be implemented manually by the driver. The value returned should be
    /// in milliseconds and must include current hours, minutes, seconds and millis.
    fn now(&mut self) -> u32;

    /// Returns the amount of time passed from a certain point. The value is expected in
    /// milliseconds and includes hours, minutes, seconds and millis. It is better to implement a
    /// faster version in the driver.
    ///
    /// Return Some(amount) in milliseconds which is amount of time passed from a certain point.
    /// If the provided time values is in the future, returns None.
    fn dt(&mut self, t: u32) -> Option<u32> {
        self.now().checked_sub(t)
    }

    /// Gets a current year. Must be implemented by the driver.
    fn year(&mut self) -> u16;

    /// Gets a current month. Must be implemented by the driver.
    fn month(&mut self) -> u8;
    
    /// Gets a current day. Must be implemented by the driver.
    fn day(&mut self) -> u8;

    /// Gets the hours. It is better to implement a faster version in the driver.
    fn hours(&mut self) -> u8 {
        (self.now() >> 24) as u8
    }

    /// Gets the minutes. It is better to implement a faster version in the driver.
    fn minutes(&mut self) -> u8 {
        (self.now() >> 16) as u8
    }

    /// Gets the seconds. It is better to implement a faster version in the driver.
    fn seconds(&mut self) -> u8 {
        (self.now() >> 8) as u8
    }

    /// Gets the milliseconds. It is better to implement a faster version in the driver.
    fn millis(&mut self) -> u8 {
        self.now() as u8
    }
}

impl_driver!(Box<dyn ClockDriver>);
