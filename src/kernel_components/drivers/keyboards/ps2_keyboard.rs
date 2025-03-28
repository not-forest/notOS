/// A driver module for PS/2 Keyboard.

use crate::{kernel_components::{arch_x86_64::controllers::PS2, drivers::Driver, os::OSChar, sync::Mutex}, single};
use super::{keyboard::KeyboardDriver, layouts::US104KEY, Key, KeyCode, KeyboardLayout, Modifiers, ScanCode, ScancodeError, ScancodeSet1, ScancodeSetTrait};
use core::fmt::Debug;

/// A driver for a PS/2 keyboard.
/// 
/// This driver provides a support for receiving inputs from the PS/2 keyboard and translate
/// them as the character or as a keycode which says about the key and it's state. Different
/// layouts and scan codes can be used.
#[derive(Debug)]
pub struct PS2Keyboard<S, L> where 
    S: ScancodeSetTrait + Debug + Clone + Copy,
    L: KeyboardLayout,
{
    modifiers: Modifiers,
    current_code: ScanCode<S>,
    layout: L,
    controller: PS2,
}

impl<S: ScancodeSetTrait + Debug + Clone + Copy, L: KeyboardLayout> PS2Keyboard<S, L> {
    /// Creates a new instance of 'PS2Keyboard'.
    /// 
    /// The provided layout must be the used layout on this keyboard.
    #[inline]
    pub const fn new(scancode_set: S, layout: L) -> Self {
        Self {
            modifiers: Modifiers {
                lshift: false,
                rshift: false,
                lctrl: false,
                rctrl: false,
                numlock: true,
                capslock: false,
                lalt: false,
                ralt: false,
                hctrl: false,
            },
            current_code: ScanCode::new(scancode_set),
            layout,
            controller: PS2::new(),
        }
    }

    /// Scans the scancode and returns the keycode of the pressed/released key.
    /// 
    /// This function only returns the keycode and the provided state of the key. It will
    /// not return any kind of character representation of the pressed key.
    pub fn scan_key(&mut self, scancode: u8) -> Result<Option<KeyCode>, ScancodeError> {
        if let Some(keycode) = self.current_code.input(scancode)? {
            self.modifiers.configure(keycode);

            return Ok(Some(keycode))
        }

        Ok(None)
    }

    /// Gets the keycode as an input and returns a character with applied modifiers.
    /// 
    /// This function will return the character if the provided keycode is something that
    /// can be represented as one, or return nothing if not.
    pub fn scan_char(&mut self, keycode: KeyCode) -> Option<char> {
        self.layout.map_keycode(&self.modifiers, keycode)
    }

    /// Clears the buffer of scancode.
    /// 
    /// This can be used if the keyboard will be interrupted with something related to the same
    /// keyboard. It prevents the timeouts related to special keys and uppercase letters in the
    /// second scan code set.
    pub fn clear(&mut self) {
        use super::scancodes::ScanCodeState;

        unsafe { self.current_code.change_state( ScanCodeState::Filled ) }
    }
}

impl Default for PS2Keyboard<ScancodeSet1, US104KEY> {
    fn default() -> Self {
        Self::new(ScancodeSet1, US104KEY)
    }
}

impl<S: ScancodeSetTrait + Debug + Clone + Copy, L: KeyboardLayout> KeyboardDriver for PS2Keyboard<S, L> {
    fn read(&mut self) -> Option<OSChar> {
        let scancode = self.controller.read_data();

        if let Ok(Some(keycode)) = self.scan_key(scancode) {
            return Some(OSChar::new(self.scan_char(keycode), keycode))
        }
        None
    }
}
