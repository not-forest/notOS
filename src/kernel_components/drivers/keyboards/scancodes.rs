/// A module that provides support for different scan sets.

use super::keyboard::{Key, KeyCode};
use core::error::Error;
use core::fmt::Display;

/// This specific code will be provided in the second scancode set when the key is released.
/// The next code that goes after this will be the key that is released.
pub const KEY_RELEASED: u8 = 0xf0;
/// This specific code will be provided when the extra key will be pressed. Basically those
/// extra keys generate this code with the code of the key itself.
pub const EXTENDED_BYTE0: u8 = 0xe0;
/// This specific code will be provided when the extra key from the second set will be 
/// pressed. Basically those extra keys generate this code with the code of the key itself.
/// The second set consists only of pause key.
pub const EXTENDED_BYTE1: u8 = 0xe1;

/// This struct is a scan code that hides the logic behind the interrupts of the keyboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ScanCode<S> where S: ScancodeSetTrait {
    state: ScanCodeState,
    set: S,
}

impl<S: ScancodeSetTrait> ScanCode<S> {
    /// Creates the new instance of Scancode.
    /// 
    /// Usually only one scancode struct must be used per keyboard.
    #[inline]
    pub const fn new(scancode_set: S) -> Self {
        Self {
            state: ScanCodeState::Filled,
            set: scancode_set,
        }
    }

    /// Takes the next code as an input and makes some inner assumptions based on the input
    /// code.
    /// 
    /// If the provided scancode from the interrupt is not something special, the returned
    /// value will be the KeyCode that corresponds that scancode. If the interrupt is
    /// something unique, then the state will be changed to await until the next interrupt
    /// arrives that will be either a normal value, or another special interrupt.
    /// 
    /// # Scancode set 1
    /// 
    /// With the first scancode set, the scancode can enter the await mode only once, when
    /// the interrupt is either 'EXTENDED_BYTE0' or 'EXTENDED_BYTE1'. The next byte is guaranteed
    /// to be the valuable key.
    /// 
    /// # Scancode set 2
    /// 
    /// With the second scancode set, the scancode can enter the await mode up to two times,
    /// when the interrupt of a 'EXTENDED_BYTE0' or 'EXTENDED_BYTE1' is followed by 
    /// 'KEY_RELEASED'. The third interrupt is guaranteed to be the valuable key.
    /// 
    /// # Custom set
    /// 
    /// For custom sets, the parse method must be created.
    pub fn input(&mut self, code: u8) -> Result<Option<KeyCode>, ScancodeError> {
        self.set.parse(
            &mut self.state,
            code,
        )
    }

    /// Changes the state of the scancode manually.
    #[inline]
    pub unsafe fn change_state(&mut self, state: ScanCodeState) {
        self.state = state;
    }
}

/// States of the scan code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ScanCodeState {
    /// The input is some custom interrupt that could mean any of the following:
    /// - EXTENDED_BYTE0 (means that the next key will be from the extended set);
    /// - EXTENDED_BYTE1 (means that the next key will be from the extended set 2);
    /// - KEY_RELEASED (means that the next key will be a released key);
    Await(u8),
    /// A normal data that is actually related to some key.
    Filled,
}

/// This trait provides a way to easy implement new scancode (if there will be any).
pub trait ScancodeSetTrait {
    /// Translates the scancode into related key.
    fn translate(scancode: u8) -> Result<Key, ScancodeError>;
    /// Translated the scancode with extended byte (E0) into related key.
    fn translate_e0(scancode: u8) -> Result<Key, ScancodeError>;
    /// Translated the scancode with extended byte (E1) into related key.
    fn translate_e1(scancode: u8) -> Result<Key, ScancodeError>;

    /// Parses the input code and delivers the output based on the current state.
    fn parse(&self, state: &mut ScanCodeState, code: u8) -> Result<Option<KeyCode>, ScancodeError>;
}

/// A first set of scan codes.
/// 
/// This set is being used by the controller and will be returned usually from it. Normally the
/// keyboard itself will use the second scancode which is a standard now, but the device controller
/// will translate the output into the first scancode set for compatibility.
#[derive(Debug, Clone, Copy)]
pub struct ScancodeSet1;

impl ScancodeSetTrait for ScancodeSet1 {
    fn translate(scancode: u8) -> Result<Key, ScancodeError> {
        use Key::*;

        match scancode {
            0x01 => Ok(Escape),
            0x02 => Ok(Key1),
            0x03 => Ok(Key2),
            0x04 => Ok(Key3),
            0x05 => Ok(Key4),
            0x06 => Ok(Key5),
            0x07 => Ok(Key6),
            0x08 => Ok(Key7),
            0x09 => Ok(Key8),
            0x0A => Ok(Key9),
            0x0B => Ok(Key0),
            0x0C => Ok(OemMinus),
            0x0D => Ok(OemPlus),
            0x0E => Ok(Backspace),
            0x0F => Ok(Tab),
            0x10 => Ok(Q),
            0x11 => Ok(W),
            0x12 => Ok(E),
            0x13 => Ok(R),
            0x14 => Ok(T),
            0x15 => Ok(Y),
            0x16 => Ok(U),
            0x17 => Ok(I),
            0x18 => Ok(O),
            0x19 => Ok(P),
            0x1A => Ok(Oem4),
            0x1B => Ok(Oem6),
            0x1C => Ok(Return),
            0x1D => Ok(LControl),
            0x1E => Ok(A),
            0x1F => Ok(S),
            0x20 => Ok(D),
            0x21 => Ok(F),
            0x22 => Ok(G),
            0x23 => Ok(H),
            0x24 => Ok(J),
            0x25 => Ok(K),
            0x26 => Ok(L),
            0x27 => Ok(Oem1),
            0x28 => Ok(Oem3),
            0x29 => Ok(Oem8),
            0x2A => Ok(LShift),
            0x2B => Ok(Oem7),
            0x2C => Ok(Z),
            0x2D => Ok(X),
            0x2E => Ok(C),
            0x2F => Ok(V),
            0x30 => Ok(B),
            0x31 => Ok(N),
            0x32 => Ok(M),
            0x33 => Ok(OemComma),
            0x34 => Ok(OemPeriod),
            0x35 => Ok(Oem2),
            0x36 => Ok(RShift),
            0x37 => Ok(NumpadMultiply),
            0x38 => Ok(LAlt),
            0x39 => Ok(Spacebar),
            0x3A => Ok(CapsLock),
            0x3B => Ok(F1),
            0x3C => Ok(F2),
            0x3D => Ok(F3),
            0x3E => Ok(F4),
            0x3F => Ok(F5),
            0x40 => Ok(F6),
            0x41 => Ok(F7),
            0x42 => Ok(F8),
            0x43 => Ok(F9),
            0x44 => Ok(F10),
            0x45 => Ok(NumpadLock),
            0x46 => Ok(ScrollLock),
            0x47 => Ok(Numpad7),
            0x48 => Ok(Numpad8),
            0x49 => Ok(Numpad9),
            0x4A => Ok(NumpadSubtract),
            0x4B => Ok(Numpad4),
            0x4C => Ok(Numpad5),
            0x4D => Ok(Numpad6),
            0x4E => Ok(NumpadAdd),
            0x4F => Ok(Numpad1),
            0x50 => Ok(Numpad2),
            0x51 => Ok(Numpad3),
            0x52 => Ok(Numpad0),
            0x53 => Ok(NumpadPeriod),
            0x54 => Ok(SystemRequest),
            0x56 => Ok(Oem5),
            0x57 => Ok(F11),
            0x58 => Ok(F12),
            _ => Err(ScancodeError(scancode)),
        }
    }

    fn translate_e0(scancode: u8) -> Result<Key, ScancodeError> {
        use Key::*;

        match scancode {
            0x10 => Ok(PrevTrack),
            0x19 => Ok(NextTrack),
            0x1C => Ok(NumpadEnter),
            0x1D => Ok(RControl),
            0x20 => Ok(Mute),
            0x21 => Ok(Calculator),
            0x22 => Ok(Play),
            0x24 => Ok(Stop),
            0x2A => Ok(RAlt2),
            0x2E => Ok(VolumeDown),
            0x30 => Ok(VolumeUp),
            0x32 => Ok(WWWHome),
            0x35 => Ok(NumpadDivide),
            0x37 => Ok(PrintScreen),
            0x38 => Ok(RAltGr),
            0x47 => Ok(Home),
            0x48 => Ok(ArrowUp),
            0x49 => Ok(PageUp),
            0x4B => Ok(ArrowLeft),
            0x4D => Ok(ArrowRight),
            0x4F => Ok(End),
            0x50 => Ok(ArrowDown),
            0x51 => Ok(PageDown),
            0x52 => Ok(Insert),
            0x53 => Ok(Delete),
            0x5B => Ok(LSuper),
            0x5C => Ok(RSuper),
            0x5D => Ok(Apps),
            // 0x5E ACPI Power
            // 0x5F ACPI Sleep
            // 0x63 ACPI Wake
            // 0x65 WWW Search
            // 0x66 WWW Favourites
            // 0x67 WWW Refresh
            // 0x68 WWW Stop
            // 0x69 WWW Forward
            // 0x6A WWW Back
            // 0x6B My Computer
            // 0x6C Email
            // 0x6D Media Select
            0x70 => Ok(Oem11),
            0x73 => Ok(Oem12),
            0x79 => Ok(Oem10),
            0x7B => Ok(Oem9),
            0x7D => Ok(Oem13),
            _ => Err(ScancodeError(scancode)),
        }
    }

    fn translate_e1(scancode: u8) -> Result<Key, ScancodeError> {
        match scancode {
            0x1D => Ok(Key::RControl2),
            _ => Err(ScancodeError(scancode)),
        }
    }

    fn parse(&self, state: &mut ScanCodeState, code: u8) -> Result<Option<KeyCode>, ScancodeError> {
        use ScanCodeState::*;

        match state {
            Filled => {
                match code {
                    EXTENDED_BYTE0 => {
                        *state = Await(EXTENDED_BYTE0);
                        Ok(None)
                    },
                    EXTENDED_BYTE1 => {
                        *state = Await(EXTENDED_BYTE1);
                        Ok(None)
                    },
                    0x80..=0xff => Ok(Some(
                        KeyCode::new(
                            Self::translate(code - 0x80)?,
                            false,
                        )
                    )),
                    _ => Ok(Some(
                        KeyCode::new(
                            Self::translate(code)?,
                            true,
                        )
                    )),
                }
            },
            Await(command) => {
                match command.clone() {
                    EXTENDED_BYTE0 => {
                        match code {
                            0x80..=0xff => {
                                *state = Filled;
                                Ok(Some(
                                    KeyCode::new(
                                        Self::translate_e0(code - 0x80)?,
                                        false,
                                    )
                                ))
                            },
                            _ => {
                                *state = Filled;
                                Ok(Some(
                                    KeyCode::new(
                                        Self::translate_e0(code)?,
                                        true,
                                    )
                                ))
                            },
                        }
                    }
                    EXTENDED_BYTE1 => {
                        match code {
                            0x80..=0xff => {
                                *state = Filled;
                                Ok(Some(
                                    KeyCode::new(
                                        Self::translate_e1(code - 0x80)?,
                                        false,
                                    )
                                ))
                            },
                            _ => {
                                *state = Filled;
                                Ok(Some(
                                    KeyCode::new(
                                        Self::translate_e1(code)?,
                                        true,
                                    )
                                ))
                            },
                        }
                    },
                    _ => unreachable!(),
                }
            },
        }
    }
}

/// The second scan code set, which is a standard for most of the keyboards.
#[derive(Debug, Clone, Copy)]
pub struct ScancodeSet2;

impl ScancodeSetTrait for ScancodeSet2 {
    fn translate(scancode: u8) -> Result<Key, ScancodeError> {
        use Key::*;

        match scancode {
            0x00 => Ok(TooManyKeys),
            0x01 => Ok(F9),
            0x03 => Ok(F5),
            0x04 => Ok(F3),
            0x05 => Ok(F1),
            0x06 => Ok(F2),
            0x07 => Ok(F12),
            0x09 => Ok(F10),
            0x0A => Ok(F8),
            0x0B => Ok(F6),
            0x0C => Ok(F4),
            0x0D => Ok(Tab),
            0x0E => Ok(Oem8),
            0x11 => Ok(LAlt),
            0x12 => Ok(LShift),
            0x13 => Ok(Oem11),
            0x14 => Ok(LControl),
            0x15 => Ok(Q),
            0x16 => Ok(Key1),
            0x1A => Ok(Z),
            0x1B => Ok(S),
            0x1C => Ok(A),
            0x1D => Ok(W),
            0x1E => Ok(Key2),
            0x21 => Ok(C),
            0x22 => Ok(X),
            0x23 => Ok(D),
            0x24 => Ok(E),
            0x25 => Ok(Key4),
            0x26 => Ok(Key3),
            0x29 => Ok(Spacebar),
            0x2A => Ok(V),
            0x2B => Ok(F),
            0x2C => Ok(T),
            0x2D => Ok(R),
            0x2E => Ok(Key5),
            0x31 => Ok(N),
            0x32 => Ok(B),
            0x33 => Ok(H),
            0x34 => Ok(G),
            0x35 => Ok(Y),
            0x36 => Ok(Key6),
            0x3A => Ok(M),
            0x3B => Ok(J),
            0x3C => Ok(U),
            0x3D => Ok(Key7),
            0x3E => Ok(Key8),
            0x41 => Ok(OemComma),
            0x42 => Ok(K),
            0x43 => Ok(I),
            0x44 => Ok(O),
            0x45 => Ok(Key0),
            0x46 => Ok(Key9),
            0x49 => Ok(OemPeriod),
            0x4A => Ok(Oem2),
            0x4B => Ok(L),
            0x4C => Ok(Oem1),
            0x4D => Ok(P),
            0x4E => Ok(OemMinus),
            0x51 => Ok(Oem12),
            0x52 => Ok(Oem3),
            0x54 => Ok(Oem4),
            0x55 => Ok(OemPlus),
            0x58 => Ok(CapsLock),
            0x59 => Ok(RShift),
            0x5A => Ok(Return),
            0x5B => Ok(Oem6),
            0x5D => Ok(Oem7),
            0x61 => Ok(Oem5),
            0x64 => Ok(Oem10),
            0x66 => Ok(Backspace),
            0x67 => Ok(Oem9),
            0x69 => Ok(Numpad1),
            0x6A => Ok(Oem13),
            0x6B => Ok(Numpad4),
            0x6C => Ok(Numpad7),
            0x70 => Ok(Numpad0),
            0x71 => Ok(NumpadPeriod),
            0x72 => Ok(Numpad2),
            0x73 => Ok(Numpad5),
            0x74 => Ok(Numpad6),
            0x75 => Ok(Numpad8),
            0x76 => Ok(Escape),
            0x77 => Ok(NumpadLock),
            0x78 => Ok(F11),
            0x79 => Ok(NumpadAdd),
            0x7A => Ok(Numpad3),
            0x7B => Ok(NumpadSubtract),
            0x7C => Ok(NumpadMultiply),
            0x7D => Ok(Numpad9),
            0x7E => Ok(ScrollLock),
            0x7F => Ok(SystemRequest),
            0x83 => Ok(F7),
            0xAA => Ok(PowerOnTestOk),
            _ => Err(ScancodeError(scancode)),
        }
    }

    fn translate_e0(scancode: u8) -> Result<Key, ScancodeError> {
        use Key::*;

        match scancode {
            0x11 => Ok(RAltGr),
            0x12 => Ok(RAlt2),
            0x14 => Ok(RControl),
            0x15 => Ok(PrevTrack),
            0x1F => Ok(LSuper),
            0x21 => Ok(VolumeDown),
            0x23 => Ok(Mute),
            0x27 => Ok(RSuper),
            0x2B => Ok(Calculator),
            0x2F => Ok(Apps),
            0x32 => Ok(VolumeUp),
            0x34 => Ok(Play),
            0x3A => Ok(WWWHome),
            0x3B => Ok(Stop),
            0x4A => Ok(NumpadDivide),
            0x4D => Ok(NextTrack),
            0x5A => Ok(NumpadEnter),
            0x69 => Ok(End),
            0x6B => Ok(ArrowLeft),
            0x6C => Ok(Home),
            0x70 => Ok(Insert),
            0x71 => Ok(Delete),
            0x72 => Ok(ArrowDown),
            0x74 => Ok(ArrowRight),
            0x75 => Ok(ArrowUp),
            0x7A => Ok(PageDown),
            0x7C => Ok(PrintScreen),
            0x7D => Ok(PageUp),
            _ => Err(ScancodeError(scancode)),
        }
    }

    fn translate_e1(scancode: u8) -> Result<Key, ScancodeError> {
        match scancode {
            0x14 => Ok(Key::RControl2),
            _ => Err(ScancodeError(scancode)),
        }
    }

    fn parse(&self, state: &mut ScanCodeState, code: u8) -> Result<Option<KeyCode>, ScancodeError> {
        use ScanCodeState::*;
        let RELEASE_EXTENDED0 = KEY_RELEASED + 1;
        let RELEASE_EXTENDED1 = KEY_RELEASED + 2;

        match state {
            Filled => {
                match code {
                    EXTENDED_BYTE0 => {
                        *state = Await(EXTENDED_BYTE0);
                        Ok(None)
                    },
                    EXTENDED_BYTE1 => {
                        *state = Await(EXTENDED_BYTE1);
                        Ok(None)
                    },
                    KEY_RELEASED => {
                        *state = Await(KEY_RELEASED);
                        Ok(None)
                    },
                    _ => Ok(Some(
                        KeyCode::new(
                            Self::translate(code)?,
                            true,
                        )
                    )),
                }
            },
            Await(command) => {
                match command.clone() {
                    EXTENDED_BYTE0 => {
                        match code {
                            KEY_RELEASED => {
                                *state = Await(KEY_RELEASED + 1);
                                Ok(None)
                            },
                            _ => {
                                *state = Filled;
                                Ok(Some(
                                    KeyCode::new(
                                        Self::translate_e0(code)?,
                                        true,
                                    )
                                ))
                            }
                        }
                    }
                    EXTENDED_BYTE1 => {
                        match code {
                            KEY_RELEASED => {
                                *state = Await(KEY_RELEASED + 1);
                                Ok(None)
                            },
                            _ => {
                                *state = Filled;
                                Ok(Some(
                                    KeyCode::new(
                                        Self::translate_e1(code)?,
                                        true,
                                    )
                                ))
                            }
                        }
                    },
                    RELEASE_EXTENDED0 => {
                        *state = Filled;
                        Ok(Some(
                            KeyCode::new(
                                Self::translate_e0(code)?,
                                false,
                            )
                        ))
                    },
                    RELEASE_EXTENDED1 => {
                        *state = Filled;
                        Ok(Some(
                            KeyCode::new(
                                Self::translate_e1(code)?,
                                false,
                            )
                        ))
                    }
                    _ => unreachable!(),
                }
            },
        }
    }
}

#[derive(Debug)]
pub struct ScancodeError(u8);
impl Error for ScancodeError {}

impl Display for ScancodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "The obtained code is not related to the chosen scancode set: {:#x}", self.0)
    }
}