/// A module with different keyboard layouts.

use super::{KeyCode, Key, Modifiers};

/// A trait that represents a layout.
/// 
/// Layouts are made to decode the pressed or released key into the unicode and handle key
/// combinations.
pub trait KeyboardLayout {
    /// A function that decodes the keycode and provides a char output based on the current
    /// modifiers, hotkeys, layout and language.
    fn map_keycode(&self, modifiers: &Modifiers, key: KeyCode) -> Option<char>;
}

/// A layout for United States keyboard with 104 keys.
#[derive(Debug)]
pub struct US104KEY;

impl KeyboardLayout for US104KEY {
    fn map_keycode(
        &self,
        modifiers: &Modifiers,
        keycode: KeyCode,
    ) -> Option<char> {
        use Key::*;

        if !keycode.is_pressed() {
            return None
        }

        match keycode.key {
            Oem8 => {
                if modifiers.is_shifted() {
                    Some('~')
                } else {
                    Some('`')
                }
            }
            Escape => Some(0x1.into()),
            Key1 => {
                if modifiers.is_shifted() {
                    Some('!')
                } else {
                    Some('1')
                }
            }
            Key2 => {
                if modifiers.is_shifted() {
                    Some('@')
                } else {
                    Some('2')
                }
            }
            Key3 => {
                if modifiers.is_shifted() {
                    Some('#')
                } else {
                    Some('3')
                }
            }
            Key4 => {
                if modifiers.is_shifted() {
                    Some('$')
                } else {
                    Some('4')
                }
            }
            Key5 => {
                if modifiers.is_shifted() {
                    Some('%')
                } else {
                    Some('5')
                }
            }
            Key6 => {
                if modifiers.is_shifted() {
                    Some('^')
                } else {
                    Some('6')
                }
            }
            Key7 => {
                if modifiers.is_shifted() {
                    Some('&')
                } else {
                    Some('7')
                }
            }
            Key8 => {
                if modifiers.is_shifted() {
                    Some('*')
                } else {
                    Some('8')
                }
            }
            Key9 => {
                if modifiers.is_shifted() {
                    Some('(')
                } else {
                    Some('9')
                }
            }
            Key0 => {
                if modifiers.is_shifted() {
                    Some(')')
                } else {
                    Some('0')
                }
            }
            OemMinus => {
                if modifiers.is_shifted() {
                    Some('_')
                } else {
                    Some('-')
                }
            }
            OemPlus => {
                if modifiers.is_shifted() {
                    Some('+')
                } else {
                    Some('=')
                }
            }
            Backspace => Some(0x0.into()),
            Tab => Some(0x0.into()),
            Q => {
                if modifiers.is_caps() {
                    Some('Q')
                } else {
                    Some('q')
                }
            }
            W => {
                if modifiers.is_caps() {
                    Some('W')
                } else {
                    Some('w')
                }
            }
            E => {
                if modifiers.is_caps() {
                    Some('E')
                } else {
                    Some('e')
                }
            }
            R => {
                if modifiers.is_caps() {
                    Some('R')
                } else {
                    Some('r')
                }
            }
            T => {
                if modifiers.is_caps() {
                    Some('T')
                } else {
                    Some('t')
                }
            }
            Y => {
                if modifiers.is_caps() {
                    Some('Y')
                } else {
                    Some('y')
                }
            }
            U => {
                if modifiers.is_caps() {
                    Some('U')
                } else {
                    Some('u')
                }
            }
            I => {
                if modifiers.is_caps() {
                    Some('I')
                } else {
                    Some('i')
                }
            }
            O => {
                if modifiers.is_caps() {
                    Some('O')
                } else {
                    Some('o')
                }
            }
            P => {
                if modifiers.is_caps() {
                    Some('P')
                } else {
                    Some('p')
                }
            }
            Oem4 => {
                if modifiers.is_shifted() {
                    Some('{')
                } else {
                    Some('[')
                }
            }
            Oem6 => {
                if modifiers.is_shifted() {
                    Some('}')
                } else {
                    Some(']')
                }
            }
            Oem5 => {
                if modifiers.is_shifted() {
                    Some('|')
                } else {
                    Some('\\')
                }
            }
            A => {
                if modifiers.is_caps() {
                    Some('A')
                } else {
                    Some('a')
                }
            }
            S => {
                if modifiers.is_caps() {
                    Some('S')
                } else {
                    Some('s')
                }
            }
            D => {
                if modifiers.is_caps() {
                    Some('D')
                } else {
                    Some('d')
                }
            }
            F => {
                if modifiers.is_caps() {
                    Some('F')
                } else {
                    Some('f')
                }
            }
            G => {
                if modifiers.is_caps() {
                    Some('G')
                } else {
                    Some('g')
                }
            }
            H => {
                if modifiers.is_caps() {
                    Some('H')
                } else {
                    Some('h')
                }
            }
            J => {
                if modifiers.is_caps() {
                    Some('J')
                } else {
                    Some('j')
                }
            }
            K => {
                if modifiers.is_caps() {
                    Some('K')
                } else {
                    Some('k')
                }
            }
            L => {
                if modifiers.is_caps() {
                    Some('L')
                } else {
                    Some('l')
                }
            }
            Oem1 => {
                if modifiers.is_shifted() {
                    Some(':')
                } else {
                    Some(';')
                }
            }
            Oem3 => {
                if modifiers.is_shifted() {
                    Some('"')
                } else {
                    Some('\\')
                }
            }
            // Enter gives LF, not CRLF or CR
            Return => Some(10.into()),
            Z => {
                if modifiers.is_caps() {
                    Some('Z')
                } else {
                    Some('z')
                }
            }
            X => {
                if modifiers.is_caps() {
                    Some('X')
                } else {
                    Some('x')
                }
            }
            C => {
                if modifiers.is_caps() {
                    Some('C')
                } else {
                    Some('c')
                }
            }
            V => {
                if modifiers.is_caps() {
                    Some('V')
                } else {
                    Some('v')
                }
            }
            B => {
                if modifiers.is_caps() {
                    Some('B')
                } else {
                    Some('b')
                }
            }
            N => {
                if modifiers.is_caps() {
                    Some('N')
                } else {
                    Some('n')
                }
            }
            M => {
                if modifiers.is_caps() {
                    Some('M')
                } else {
                    Some('m')
                }
            }
            OemComma => {
                if modifiers.is_shifted() {
                    Some('<')
                } else {
                    Some(',')
                }
            }
            OemPeriod => {
                if modifiers.is_shifted() {
                    Some('>')
                } else {
                    Some('.')
                }
            }
            Oem2 => {
                if modifiers.is_shifted() {
                    Some('?')
                } else {
                    Some('/')
                }
            }
            Spacebar => Some(' '),
            Delete => Some(127.into()),
            NumpadDivide => Some('/'),
            NumpadMultiply => Some('*'),
            NumpadSubtract => Some('-'),
            Numpad7 => {
                if modifiers.numlock {
                    Some('7')
                } else {
                    None
                }
            }
            Numpad8 => {
                if modifiers.numlock {
                    Some('8')
                } else {
                    None
                }
            }
            Numpad9 => {
                if modifiers.numlock {
                    Some('9')
                } else {
                    None
                }
            }
            NumpadAdd => Some('+'),
            Numpad4 => {
                if modifiers.numlock {
                    Some('4')
                } else {
                    None
                }
            }
            Numpad5 => Some('5'),
            Numpad6 => {
                if modifiers.numlock {
                    Some('6')
                } else {
                    None
                }
            }
            Numpad1 => {
                if modifiers.numlock {
                    Some('1')
                } else {
                    None
                }
            }
            Numpad2 => {
                if modifiers.numlock {
                    Some('2')
                } else {
                    None
                }
            }
            Numpad3 => {
                if modifiers.numlock {
                    Some('3')
                } else {
                    None
                }
            }
            Numpad0 => {
                if modifiers.numlock {
                    Some('0')
                } else {
                    None
                }
            }
            NumpadPeriod => {
                if modifiers.numlock {
                    Some('.')
                } else {
                    Some(127.into())
                }
            }
            NumpadEnter => Some(10.into()),
            _ => None,
        }
    }
}
