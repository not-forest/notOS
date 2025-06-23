use core::{any::Any, ops::DerefMut};

/// A module for all build-in libraries.

use alloc::{boxed::Box, collections::BTreeMap, string::{String, ToString}};
use keyboards::keyboard::KeyboardDriver;
use timers::ClockDriver;

use crate::{debug, single};

pub type DriverResult<T> = Result<T, DriverError>;
pub type DriverName = String;

/// A custom trait that marks out something as a driver.
/// 
/// All sub-driver category must implement this trait for downcast it from this supertrait. This
/// way many drivers could be stored within the binary tree.
pub trait Driver: Any {
    /// Returns information about the driver.
    fn info(&self) -> DriverInfo;
}

/// A default driver manager.
///
/// During OS boot drivers must be loaded into their corresponding pointers.
single! {
    pub mut DRIVER_MANAGER: DriverManager = DriverManager::default()
}

/// Driver information.
///
/// This information allows to define several things for kernel and external modules:
/// - For which peripheral this driver is implemented;
/// - What name does this driver has (used to not store drivers with same names.);
#[derive(Debug)]
pub struct DriverInfo {
    pub r#type: DriverType,
    pub name: DriverName,
}

/// Main structure that allows to interface all other drivers.
///
/// Every part of the kernel that wish to obtain the driver would use a static global driver
/// manager instance to obtain a reference to real drivers. This structure does not ensure any
/// mutual exclusion or synchronization of any kind, therefore it is must be implemented within the
/// drivers logic.
#[derive(Default)]
pub struct DriverManager {
    drivers: BTreeMap<String, Box<dyn Driver>>,
}

impl DriverManager {
    /// Obtains the currently loaded driver based on it's name.
    ///
    /// # Returns
    ///
    /// Returns a mutable reference if it is loaded to the kernel.
    pub fn driver(&mut self, name: DriverName) -> Option<&mut dyn Driver> {
        self.drivers.get_mut(&name)
            .and_then(|driver| driver.)
    }

    /// Loads the driver into the driver manager.
    ///
    /// # Returns
    ///
    /// An error if such driver already exist. A string with driver's name if it was loaded
    /// successfully.
    pub fn load<T>(&mut self, driver: T, name: DriverName) -> DriverResult<String> where T: Driver {
        let str = driver.info().name.to_string();
        if let Err(_) = self.drivers.try_insert(name, Box::new(driver)) {
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
        if let Some((name, _)) = self.drivers.iter().find(|(k, v)| v.info().name == name) {
            self.drivers.remove(&name.clone());
            debug!("Mod \"{}\" is unloaded", name.as_str());
            Ok(())
        } else {
            Err(DriverError::NotLoaded)
        }
    }
}

/// Defines different driver types for query.
///
/// This can be thought as a tag, which allows to easily locate the required loaded driver inside
/// the binary tree structure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DriverType {
    Keyboard, 
    Mouse, 
    Clock, 
    Interrupt
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

    pub use keyboard::{Key, KeyCode, Modifiers, KeyboardDriver};
    pub use scancodes::{ScanCode, ScancodeError, ScancodeSetTrait, ScancodeSet1, ScancodeSet2};
    pub use layouts::KeyboardLayout;

    pub use ps2_keyboard::PS2Keyboard;
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

/// Interrupts
pub mod interrupts {
    /// Interrupt controller driver. Either APIC or PIC, LPIC etc.
    pub mod int_ctrl;

    pub use int_ctrl::InterruptControllerDriver;
}
