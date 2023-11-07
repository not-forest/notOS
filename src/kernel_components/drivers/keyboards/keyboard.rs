/// A module that represents a Keyboard trait and all it's needed components.

use super::{
    PS2Keyboard, ScancodeSet1,
    layouts::US104KEY,
};
use crate::kernel_components::drivers::Driver;
use crate::kernel_components::sync::Mutex;
use crate::single;

/// A global keyboard static variable.
single! {
    pub GLOBAL_KEYBORD: Mutex<PS2Keyboard<ScancodeSet1, US104KEY>> = Mutex::new(PS2Keyboard::new(
        ScancodeSet1,
        US104KEY,
    ));
}

/// The keys that can be pressed by any keyboard.
/// 
/// This enum must be used with custom scancode sets, to describe each individual scancode
/// to a understandable character.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Key {
    // Row 1
    /// Escape key
    Escape,
    /// Function Key F1
    F1,
    /// Function Key F2
    F2,
    /// Function Key F3
    F3,
    /// Function Key F4
    F4,
    /// Function Key F5
    F5,
    /// Function Key F6
    F6,
    /// Function Key F7
    F7,
    /// Function Key F8
    F8,
    /// Function Key F9
    F9,
    /// Function Key F10
    F10,
    /// Function Key F11
    F11,
    /// Function Key F12
    F12,
    /// Print Screen Key
    PrintScreen,
    /// The System request key (Alt + PrintScreen)
    SystemRequest,
    /// The Scroll Lock key
    ScrollLock,
    /// The Pause/Break key
    PauseBreak,

    // Row 2
    /// Usually called Tilda.
    Oem8,
    /// Number Line 1
    Key1,
    /// Number Line 2
    Key2,
    /// Number Line 3
    Key3,
    /// Number Line 4
    Key4,
    /// Number Line 5
    Key5,
    /// Number Line 6
    Key6,
    /// Number Line 7
    Key7,
    /// Number Line 8
    Key8,
    /// Number Line 9
    Key9,
    /// Number Line 0
    Key0,
    /// Minus/Underscore Key
    OemMinus,
    /// Equals/Plus Key
    OemPlus,
    /// Backspace
    Backspace,
    /// Insert key. Top Left of the Extended Block
    Insert,
    /// Home key. Top Middle of the Extended Block
    Home,
    /// PageUp key. Top Right of the Extended Block
    PageUp,
    /// The Num Lock key
    NumpadLock,
    /// The Numpad Divide (or Slash) key
    NumpadDivide,
    /// The Numpad Multiple (or Star) key
    NumpadMultiply,
    /// The Numpad Subtract (or Minus) key
    NumpadSubtract,

    // Row 3
    /// The Tab Key
    Tab,
    /// Letter Q
    Q,
    /// Letter W
    W,
    /// Letter E
    E,
    /// Letter R
    R,
    /// Letter T
    T,
    /// Letter Y
    Y,
    /// Letter U
    U,
    /// Letter I
    I,
    /// Letter O
    O,
    /// Letter P
    P,
    /// US ANSI Left-Square-Bracket key
    Oem4,
    /// US ANSI Right-Square-Bracket key
    Oem6,
    /// US ANSI Backslash Key / UK ISO Backslash Key
    Oem5,
    /// The UK/ISO Hash/Tilde key (ISO layout only)
    Oem7,
    /// The Delete key - bottom Left of the Extended Block
    Delete,
    /// The End key - bottom Middle of the Extended Block
    End,
    /// The Page Down key - -bottom Right of the Extended Block
    PageDown,
    /// The Numpad 7/Home key
    Numpad7,
    /// The Numpad 8/Up Arrow key
    Numpad8,
    /// The Numpad 9/Page Up key
    Numpad9,
    /// The Numpad Add/Plus key
    NumpadAdd,

    // Row 4
    /// Caps Lock
    CapsLock,
    /// Letter A
    A,
    /// Letter S
    S,
    /// Letter D
    D,
    /// Letter F
    F,
    /// Letter G
    G,
    /// Letter H
    H,
    /// Letter J
    J,
    /// Letter K
    K,
    /// Letter L
    L,
    /// The US ANSI Semicolon/Colon key
    Oem1,
    /// The US ANSI Single-Quote/At key
    Oem3,
    /// The Return Key
    Return,
    /// The Numpad 4/Left Arrow key
    Numpad4,
    /// The Numpad 5 Key
    Numpad5,
    /// The Numpad 6/Right Arrow key
    Numpad6,

    // Row 5
    /// Left Shift
    LShift,
    /// Letter Z
    Z,
    /// Letter X
    X,
    /// Letter C
    C,
    /// Letter V
    V,
    /// Letter B
    B,
    /// Letter N
    N,
    /// Letter M
    M,
    /// US ANSI `,<` key
    OemComma,
    /// US ANSI `.>` Key
    OemPeriod,
    /// US ANSI `/?` Key
    Oem2,
    /// Right Shift
    RShift,
    /// The up-arrow in the inverted-T
    ArrowUp,
    /// Numpad 1/End Key
    Numpad1,
    /// Numpad 2/Arrow Down Key
    Numpad2,
    /// Numpad 3/Page Down Key
    Numpad3,
    /// Numpad Enter
    NumpadEnter,

    // Row 6
    /// The left-hand Control key
    LControl,
    /// The left-hand 'Super' or 'Windows' key
    LSuper,
    /// The left-hand Alt key
    LAlt,
    /// The Space Bar
    Spacebar,
    /// The right-hand AltGr key
    RAltGr,
    /// The right-hand 'Super' or 'Windows' key
    RSuper,
    /// The 'Apps' key (aka 'Menu' or 'Right-Click')
    Apps,
    /// The right-hand Control key
    RControl,
    /// The left-arrow in the inverted-T
    ArrowLeft,
    /// The down-arrow in the inverted-T
    ArrowDown,
    /// The right-arrow in the inverted-T
    ArrowRight,
    /// The Numpad 0/Insert Key
    Numpad0,
    /// The Numppad Period/Delete Key
    NumpadPeriod,

    // Extra
    /// Extra JIS key (0x7B)
    Oem9,
    /// Extra JIS key (0x79)
    Oem10,
    /// Extra JIS key (0x70)
    Oem11,
    /// Extra JIS symbol key (0x73)
    Oem12,
    /// Extra JIS symbol key (0x7D)
    Oem13,
    /// Multi-media keys - Previous Track
    PrevTrack,
    /// Multi-media keys - Next Track
    NextTrack,
    /// Multi-media keys - Volume Mute Toggle
    Mute,
    /// Multi-media keys - Open Calculator
    Calculator,
    /// Multi-media keys - Play
    Play,
    /// Multi-media keys - Stop
    Stop,
    /// Multi-media keys - Increase Volume
    VolumeDown,
    /// Multi-media keys - Decrease Volume
    VolumeUp,
    /// Multi-media keys - Open Browser
    WWWHome,
    /// Sent when the keyboard boots
    PowerOnTestOk,
    /// Sent by the keyboard when too many keys are pressed
    TooManyKeys,
    /// Used as a 'hidden' Right Control Key (Pause = RControl2 + Num Lock)
    RControl2,
    /// Used as a 'hidden' Right Alt Key (Print Screen = RAlt2 + PrntScr)
    RAlt2,

    /// A custom extra key that can be specific for certain keyboards.
    Custom(u8),
}

/// The set of modifier keys you have on a keyboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Modifiers {
    /// The left shift key is down
    pub lshift: bool,
    /// The right shift key is down
    pub rshift: bool,
    /// The left control key is down
    pub lctrl: bool,
    /// The right control key is down
    pub rctrl: bool,
    /// The Num Lock toggle is on
    pub numlock: bool,
    /// The caps lock toggle is on
    pub capslock: bool,
    /// The left alt key is down
    pub lalt: bool,
    /// The right alt key is down
    pub ralt: bool,
    /// Special 'hidden' control key is down (used when you press Pause)
    pub hctrl: bool,
}

impl Modifiers {
    /// Configures the modifiers based on the input keycode.
    pub fn configure(&mut self, keycode: KeyCode) {
        use Key::*;

        match keycode.key {
            NumpadLock => {
                if keycode.is_pressed() {
                    self.numlock = !self.numlock;
                }
            },
            CapsLock => {
                if keycode.is_pressed() {
                    self.capslock = !self.capslock;
                }                  
            },
            LControl => {
                if keycode.is_pressed() {
                    self.lctrl = true;
                } else {
                    self.lctrl = false;                        
                }
            },
            RControl => {
                if keycode.is_pressed() {
                    self.rctrl = true;
                } else {
                    self.rctrl = false;                        
                }
            },
            LShift => {
                if keycode.is_pressed() {
                    self.lshift = true;
                } else {
                    self.lshift = false;                        
                }
            },
            RShift => {
                if keycode.is_pressed() {
                    self.rshift = true;
                } else {
                    self.rshift = false;                        
                }
            },
            LAlt => {
                if keycode.is_pressed() {
                    self.lalt = true;
                } else {
                    self.lalt = false;                        
                }
            },
            RAlt => {
                if keycode.is_pressed() {
                    self.ralt = true;
                } else {
                    self.ralt = false;                        
                }
            },
            RControl2 => {
                if keycode.is_pressed() {
                    self.hctrl = true;
                } else {
                    self.hctrl = false;                        
                }
            },
            _ => (),
        }
    }

    pub const fn is_shifted(&self) -> bool {
        self.lshift | self.rshift
    }

    pub const fn is_ctrl(&self) -> bool {
        self.lctrl | self.rctrl
    }

    pub const fn is_alt(&self) -> bool {
        self.lalt | self.ralt
    }

    pub const fn is_altgr(&self) -> bool {
        self.ralt | (self.lalt & self.is_ctrl())
    }

    pub const fn is_caps(&self) -> bool {
        self.is_shifted() ^ self.capslock
    }
}

impl Default for Modifiers {
    fn default() -> Self {
        Self {
            lshift: false,
            rshift: false,
            lctrl: false,
            rctrl: false,
            numlock: true,
            capslock: false,
            lalt: false,
            ralt: false,
            hctrl: false,
        }
    }
}

/// A struct that represent a read keycode that is ready to be parsed into ascii or unicode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct KeyCode {
    pub key: Key,
    is_pressed: bool,
}

impl KeyCode {
    /// Creates a new instance of a KeyCode.
    #[inline]
    pub const fn new(key: Key, is_pressed: bool) -> Self {
        Self {
            key, is_pressed
        }
    }

    /// Provides info about if the key is pressed or released.
    #[inline]
    pub const fn is_pressed(&self) -> bool {
        self.is_pressed
    }
}