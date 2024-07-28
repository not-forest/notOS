/// A module for all build-in libraries.

/// A custom trait that marks out something as a driver.
/// 
/// Each struct that represents a driver of some sort must be marked with this trait. 
pub trait Driver: Sized {
    fn change_driver(&mut self, new_driver: Self) {
        *self = new_driver;
    }
}

/// A keyboard drivers.
pub mod keyboards {
    pub mod scancodes;
    pub mod layouts;
    pub mod keyboard;

    pub mod ps2_keyboard;

    pub use keyboard::{Key, KeyCode, Modifiers, GLOBAL_KEYBORD};
    pub use scancodes::{ScanCode, ScancodeError, ScancodeSetTrait, ScancodeSet1, ScancodeSet2};
    pub use layouts::KeyboardLayout;

    pub use ps2_keyboard::PS2Keyboard;
}

/// A mouse drivers.
pub mod mouse {

}
