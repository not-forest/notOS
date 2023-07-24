// Printing strings and characters

use core::fmt;
use crate::{kernel_components::sync::Mutex, single};

const BUFFER_HEIGHT: usize = 25;
const BUFFER_WIDTH: usize = 80;

single! {
    LOGGER: Mutex<Logger> = Mutex::new(Logger {
        pos: 0,
        color_code: ColorCode::new(Color::WHITE, Color::BLACK),
        buf: unsafe { &mut *(0xb8000 as *mut Buffer) } 
    })
}

pub struct Logger {
    pos: usize,
    color_code: ColorCode,
    buf: &'static mut Buffer,
}

#[allow(dead_code)]
impl Logger {
    pub fn write(&mut self, byte: u8) {
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

    pub fn write_str(&mut self, s: &str) {
        for byte in s.bytes() {
            match byte {
                0x20..=0x7e | b'\n' => self.write(byte),
                _ => self.write(0xfe),
            }
        }
    }

    pub fn move_cursor(&mut self, row: usize, col: usize) {
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

    pub fn change_color(&mut self, fr: Color, bg: Color) {
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

impl fmt::Write for Logger {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_str(s);
        self.color_code = ColorCode::new(Color::WHITE, Color::BLACK);
        Ok(())
    }
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
struct ColorCode(u8);

impl ColorCode {
    fn new(fr: Color, bg: Color) -> ColorCode {
        ColorCode((bg as u8) << 4 | (fr as u8))
    }
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct Char {
    ascii_char: u8,
    color_code: ColorCode,
}

#[repr(transparent)]
struct Buffer {
    str: [[Char; BUFFER_WIDTH]; BUFFER_HEIGHT]
}

// MACROS
#[macro_export]
macro_rules! move_cursor {
    ($row:expr, $col:expr) => {
        LOGGER.lock().move_cursor($row, $col);
    };
}

#[macro_export]
macro_rules! print {
    ($fr:expr; $($arg:tt)*) => {
        $crate::kernel_components::vga_buffer::_coloring($fr, None);
        print!($($arg)*);
    };
    ($fr:expr; $bg:expr; $($arg:tt)*) => {
        $crate::kernel_components::vga_buffer::_coloring($fr, Some($bg));
        print!($($arg)*);
    };
    ($($arg:tt)*) => ($crate::kernel_components::vga_buffer::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => (print!("\n"));
    ($fmt:expr) => (print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => (print!(concat!($fmt, "\n"), $($arg)*));
    ($fr:expr; $fmt:expr) => {
        $crate::kernel_components::vga_buffer::_coloring($fr, None);
        println!($fmt);
    };
    ($fr:expr; $bg:expr; $fmt:expr) => {
        $crate::kernel_components::vga_buffer::_coloring($fr, Some($bg));
        println!($fmt);
    };
    ($fr:expr; $fmt:expr, $($arg:tt)*) => {
        $crate::kernel_components::vga_buffer::_coloring($fr, None);
        println!($fmt, $($arg)*);
    };
    ($fr:expr; $bg:expr; $fmt:expr, $($arg:tt)*) => {
        $crate::kernel_components::vga_buffer::_coloring($fr, Some($bg));
        println!($fmt, $($arg)*);
    };
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    LOGGER.lock().write_fmt(args).unwrap();
}

#[doc(hidden)]
pub fn _coloring(fr: Color, bg: Option<Color>) {
    if let Some(bg) = bg {
        LOGGER.lock().change_color(fr, bg);
    } else {
        LOGGER.lock().change_color(fr, Color::BLACK);
    }
}