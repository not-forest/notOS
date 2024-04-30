/// Printing strings and characters inside OS. This is the most basic way to get some output
/// from running OS or tests.
///
/// This module provides a simple Logger structure and macros for printing formatted output
/// to a VGA buffer, simulating output on the screen in a basic operating system environment.

use core::fmt;
use crate::{kernel_components::sync::Mutex, single, critical_section};

const BUFFER_HEIGHT: usize = 25;
const BUFFER_WIDTH: usize = 80;

/// Creates a lazy initialization of a static Logger instance.
single! {
    LOGGER: Mutex<Logger> = Mutex::new(Logger {
        pos: 0,
        color_code: ColorCode::new(Color::WHITE, Color::BLACK),
        buf: unsafe { &mut *(0xb8000 as *mut Buffer) } 
    })
}

/// Represents the Logger structure responsible for writing to the VGA buffer.
pub struct Logger {
    pos: usize,
    color_code: ColorCode,
    buf: &'static mut Buffer,
}

#[allow(dead_code)]
impl Logger {
    /// Writes a single byte to the VGA buffer, handling newline characters.
    pub(self) fn write(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            _ => {
                if self.pos >= BUFFER_WIDTH {
                    self.new_line()
                }

                let row: usize = BUFFER_HEIGHT - 1;
                let col: usize = self.pos;
                let color_code: ColorCode = self.color_code;
                
                self.buf.str[row][col] = Char {
                    ascii_char: byte,
                    color_code,
                };
                self.pos += 1;
            }
        }
    }

    /// Writes a string to the VGA buffer using the `write` method.
    pub(self) fn write_str(&mut self, s: &str) {
        for byte in s.bytes() {
            match byte {
                0x20..=0x7e | b'\n' => self.write(byte),
                _ => self.write(0xfe),
            }
        }
    }

    /// Moves the cursor to a specific position on the VGA buffer
    pub(self) fn move_cursor(&mut self, row: usize, col: usize) {
        if row < BUFFER_HEIGHT && col < BUFFER_WIDTH {
            self.pos = col;
            self.new_line();

            for i in 0..col {
                self.buf.str[row][i] = Char {
                    ascii_char: b' ',
                    color_code: self.color_code,
                };
            }
        }
    }

    /// Changes the color of the text in the VGA buffer.
    pub(self) fn change_color(&mut self, fr: Color, bg: Color) {
        self.color_code = ColorCode::new(fr, bg);
    }
    
    fn new_line(&mut self) {
        for row in 1..BUFFER_HEIGHT {
            for col in 0..BUFFER_WIDTH {
                let character: Char = self.buf.str[row][col];
                self.buf.str[row - 1][col] = character;
            }
        }
        self.clear_row(BUFFER_HEIGHT - 1);
        self.pos = 0;
    }

    fn clear_row(&mut self, row: usize) {
        let blank = Char {
            ascii_char: b' ',
            color_code: self.color_code,
        };
        self.buf.str[row].fill(blank);
    }

}

/// Implements the fmt::Write trait for the Logger, allowing it to be used with formatted printing macros.
impl fmt::Write for Logger {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        critical_section!(|| {
            self.write_str(s);
            Ok(())
        })
    }
}

/// All colors that are allowed to use when printing.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Color {
    BLACK = 0,
    BLUE = 1,
    GREEN = 2,
    CYAN = 3,
    RED = 4,
    MAGENTA = 5,
    BROWN = 6,
    LIGHTGRAY = 7,
    DARKGRAY = 8,
    LIGHTBLUE = 9,
    LIGHTGREEN = 10,
    LIGHTCYAN = 11,
    LIGHTRED = 12,
    PINK = 13,
    YELLOW = 14,
    WHITE = 15,
}

/// The color code structure is a number that contain a pair of background and 
/// foreground that decide which colors will be used on printing. 
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
struct ColorCode(u8);

impl ColorCode {
    /// Creates a new ColorCode, in which fr is a foreground color and bg is a background.
    /// Arguments must be values from Color enum in order to properly generate a real color pair
    fn new(fr: Color, bg: Color) -> ColorCode {
        ColorCode((bg as u8) << 4 | (fr as u8))
    }
}

/// A character representation in a VGA buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct Char {
    ascii_char: u8,
    color_code: ColorCode,
}

/// A buffer that represents a whole string for printing.
#[repr(transparent)]
struct Buffer {
    str: [[Char; BUFFER_WIDTH]; BUFFER_HEIGHT]
}

/// # Macros

/// Moves the cursor's location to given row and column. 
#[macro_export]
macro_rules! move_cursor {
    ($row:expr, $col:expr) => {
        unsafe {
            with_int_disabled(|| {
                LOGGER.lock().move_cursor($row, $col);
            });
        }
    };
}

/// Prints the content to the screen via VGA buffer. It does support coloring
/// for background and foreground. Works the same way how print! from standard
/// library would work, except for coloring.
/// 
/// If needed the coloring can be changed with Color enum. The colors are separated
/// with ; while other args are separated with a comma. The first color is always considered
/// as a foreground while the second is considered as background. By default it uses white foreground
/// and black background
/// 
/// # Examples
/// ''' 
/// use notOS::{print, Color};
/// 
/// fn main() -> ! {
///     print!(Color::BLUE; "This text's foreground will be blue. {}", "Yes it is indeed.\n");
///     print!(Color::RED; Color::GREEN; "I am red and angry, but green inside.\n");
///     print!("I am the most default dude out there.");
///
///     loop {}
/// }
/// '''
#[macro_export]
macro_rules! print {
    ($fr:expr; $bg:expr; $($arg:tt)*) => ($crate::kernel_components::vga_buffer::_print(Some($fr), Some($bg), format_args!($($arg)*)));
    ($fr:expr; $($arg:tt)*) => ($crate::kernel_components::vga_buffer::_print(Some($fr), None, format_args!($($arg)*)));
    ($($arg:tt)*) => ($crate::kernel_components::vga_buffer::_print(None, None, format_args!($($arg)*)));
}

/// Prints the content to the screen via VGA buffer and moves the cursor to new line. It does support coloring
/// for background and foreground. Works the same way how print! from standard library would work, except for coloring.
/// 
/// If needed the coloring can be changed with Color enum. The colors are separated
/// with ; while other args are separated with a comma. The first color is always considered
/// as a foreground while the second is considered as background. By default it uses white foreground
/// and black background
/// 
/// # Examples
/// ''' 
/// use notOS::{println, Color};
/// 
/// fn main() -> ! {
///     println!(Color::BLUE; "This text's foreground will be blue. {}", "Yes it is indeed.");
///     println!(Color::RED; Color::GREEN; "I am red and angry, but green inside.");
///     println!("I am the most default dude out there... maybe.");
///
///     loop {}
/// }
/// '''
#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($fmt:expr) => ($crate::print!(concat!($fmt, '\n')));
    ($fmt:expr, $($arg:tt)*) => ($crate::print!(concat!($fmt, '\n'), $($arg)*)); 
    ($fr:expr; $fmt:expr) => ($crate::print!($fr; concat!($fmt, '\n')));
    ($fr:expr; $bg:expr; $fmt:expr) => ($crate::print!($fr, $bg, concat!($fmt, '\n')));
    ($fr:expr; $fmt:expr, $($arg:tt)*) => ($crate::print!($fr; concat!($fmt, '\n'), $($arg)*));   
    ($fr:expr; $bg:expr; $fmt:expr, $($arg:tt)*) => ($crate::print!($fr; $bg; concat!($fmt, '\n'), $($arg)*)); 
}

/// Writes a warning message to the screen in yellow.
/// 
/// Works like println, but do not accept the color argument. The output will always be in yellow.
#[macro_export]
macro_rules! warn {
    () => ($crate::println!('\n'));
    ($fmt:expr) => ($crate::println!($crate::Color::YELLOW; concat!("WARNING! " ,$fmt, '\n')));
    ($fmt:expr, $($arg:tt)*) => ($crate::println!($crate::Color::YELLOW; concat!("WARNING! ", $fmt, '\n'), $($arg)*));
}

/// A fast macro to show the debug information about the item (in pretty print).
/// 
/// This macro will do nothing in release mode.
#[macro_export]
macro_rules! debug {
    ($item:tt) => (
        #[cfg(debug_assertions)]
        $crate::println!($crate::Color::LIGHTCYAN; "{:#x?}", $item)
    );
    ($($item:tt),*) => (
        $($crate::debug!($item);)*
    );
    () => ();
}

#[doc(hidden)]
pub fn _print(fr: Option<Color>, bg: Option<Color>, args: fmt::Arguments) {
    use core::fmt::Write;
    critical_section!(|| {
        _coloring(fr, bg);
        LOGGER.lock().write_fmt(args).unwrap();
        _coloring(None, None);
    });
}

#[doc(hidden)]
pub fn _coloring(fr: Option<Color>, bg: Option<Color>) {
    if let Some(bg) = bg {
        LOGGER.lock().change_color(fr.unwrap(), bg);        
    } else {
        if let Some(fr) = fr {
            LOGGER.lock().change_color(fr, Color::BLACK);
        } else {
            LOGGER.lock().change_color(Color::WHITE, Color::BLACK);
        }
    }
}
