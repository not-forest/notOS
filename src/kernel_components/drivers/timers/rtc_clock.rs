/// A clock driver based on the RTC chip.

use super::ClockDriver;
use crate::kernel_components::arch_x86_64::controllers::{RTC, CMOSAddr};
use crate::kernel_components::drivers::Driver;

/// A clock driver implementation that uses RTC as a main clock source.
///
/// All values are read from the chip itself via simple read commands. The clock is unable to
/// provide milliseconds via regular reads, therefore they are always set to 0.
pub struct RealTimeClock {
    rtc: RTC
}

impl RealTimeClock {
    /// Creates a new instance of real time clock driver.
    pub fn new () -> Self {
        Self {
            rtc: RTC::new(),
        }
    }
}

impl ClockDriver for RealTimeClock {
    fn now(&mut self) -> u32 {
          (self.rtc.read(CMOSAddr::RTC_HOURS) as u32) * 60 * 60 * 1000
        + (self.rtc.read(CMOSAddr::RTC_MINUTES) as u32) * 60 * 1000
        + (self.rtc.read(CMOSAddr::RTC_SECONDS) as u32) * 1000
    }

    fn year(&mut self) -> u16 {
        (self.rtc.read(CMOSAddr::CENTURY) as u16) * 100 +
        (self.rtc.read(CMOSAddr::RTC_YEAR) as u16)
    }

    fn month(&mut self) -> u8 {
        self.rtc.read(CMOSAddr::RTC_MONTH)
    }
    
    fn day(&mut self) -> u8 {
        self.rtc.read(CMOSAddr::RTC_DAY_OF_MONTH)
    }

    fn hours(&mut self) -> u8 {
        self.rtc.read(CMOSAddr::RTC_HOURS)
    }

    fn minutes(&mut self) -> u8 {
        self.rtc.read(CMOSAddr::RTC_MINUTES)
    }

    fn seconds(&mut self) -> u8 {
        self.rtc.read(CMOSAddr::RTC_SECONDS)
    }

    fn millis(&mut self) -> u8 {
        0
    }
}

impl_driver!(RealTimeClock);
