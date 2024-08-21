use core::{any::Any, ops::DerefMut};

/// A module for all build-in libraries.

use alloc::{boxed::Box, collections::BTreeMap, string::String};
use keyboards::keyboard::KeyboardDriver;
use timers::ClockDriver;

use crate::{debug, single};

pub type DriverResult<T> = Result<T, DriverError>;

/// A custom trait that marks out something as a driver.
/// 
/// All sub-driver category must implement this trait for downcast it from this supertrait. This
/// way many drivers could be stored within the binary tree.
pub trait Driver: Any {
    fn as_driver(&mut self) -> &mut dyn Any;  // Method to enable downcasting
    fn name(&self) -> &str;
}

/// A default driver manager.
///
/// During OS boot drivers must be loaded into their corresponding pointers.
single! {
    pub mut DRIVER_MANAGER: DriverManager = DriverManager::default()
}

/// Main structure that allows to interface all other drivers.
///
/// Every part of the kernel that wish to obtain the driver would use a static global driver
/// manager instance to obtain a reference to real drivers. This structure does not ensure any
/// mutual exclusion or synchronization of any kind, therefore it is must be implemented within the
/// drivers logic.
#[derive(Default)]
pub struct DriverManager {
    drivers: BTreeMap<DriverType, Box<dyn Driver>>,
}

impl DriverManager {
    /// Based on the driver type, downcasts a driver into it's sub-driver category for future use.
    /// Every trait that implement Driver super trait can be found this way.
    pub fn driver<T: Driver>(&mut self, dtype: DriverType) -> Option<&mut T> {
        self.drivers.get_mut(&dtype)
            .and_then(|driver| driver.as_driver().downcast_mut::<T>())
    }

    /// Loads the driver into the driver manager.
    ///
    /// # Returns
    ///
    /// An error if such driver already exist. A string with driver's name if it was loaded
    /// successfully.
    pub fn load<T>(&mut self, driver: T, dtype: DriverType) -> DriverResult<String> where T: Driver {
        let str = String::from(driver.name());
        if let Err(_) = self.drivers.try_insert(dtype, Box::new(driver)) {
            Err(DriverError::AlreadyLoaded)
        } else {
            debug!("Mod \"{}\" is loaded", str.as_str());
            Ok(str)
        }
    }

    /// Unloads the requested driver.
    ///
    /// # Returns 
    ///
    /// An error if such driver does not exist already. An Ok(()) if was deleted successfully
    pub fn unload(&mut self, name: String) -> DriverResult<()> {
        if let Some((dtype, _)) = self.drivers.iter().find(|(k, v)| v.name() == name) {
            self.drivers.remove(&dtype.clone());
            debug!("Mod \"{}\" is unloaded", name.as_str());
            Ok(())
        } else {
            Err(DriverError::NotLoaded)
        }
    }
}

macro_rules! impl_driver {
    ($t:ty) => {
        impl Driver for $t {
            fn as_driver(&mut self) -> &mut dyn core::any::Any {
                self
            }

            fn name(&self) -> &str {
                stringify!($t)
            }
        }
    };
}

/// Defines different driver types for query.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DriverType {
    Keyboard, Mouse, Clock
}

/// Error type for driver error handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DriverError {
    /// Trying to load an already loaded driver.
    AlreadyLoaded,
    /// Trying to remove an already unloaded driver.
    NotLoaded,
}

/// Keyboard drivers.
pub mod keyboards {
    /// Scancode parser for PS/2 keyboard.
    pub mod scancodes;
    /// Keyboard layouts.
    pub mod layouts;
    /// Global keyboard interface.
    pub mod keyboard;

    /// PS/2 keyboard driver.
    pub mod ps2_keyboard;

    pub use keyboard::{Key, KeyCode, Modifiers};
    pub use scancodes::{ScanCode, ScancodeError, ScancodeSetTrait, ScancodeSet1, ScancodeSet2};
    pub use layouts::KeyboardLayout;

    pub use ps2_keyboard::{PS2Keyboard, PS2_KEYBOARD};
}

/// Mouse drivers.
pub mod mouse {

}

/// Timers, counters and clocks.
pub mod timers {
    /// Global clock interface
    pub mod clock;
    /// Clock driver based on AT RTC chip.
    pub mod rtc_clock;

    pub use clock::ClockDriver;
    pub use rtc_clock::RealTimeClock;
}
