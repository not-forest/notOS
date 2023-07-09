// Printing strings and characters

use core::fmt;
use spin::Mutex;
use lazy_static::lazy_static;

const BUFFER_HEIGHT: usize = 25;
const BUFFER_WIDTH: usize = 80;

lazy_static! {
    pub static ref LOGGER: Mutex<Logger> = Mutex::new(Logger {
        pos: 0,
        color_code: ColorCode::new(Color::YELLOW, Color::BLACK),
        buf: unsafe { &mut *(0xb8000 as *mut Buffer) } 
    });
    
}

pub struct Logger {
    pos: usize,
    color_code: ColorCode,
    buf: &'static mut Buffer,
}

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
        for col in 0..BUFFER_WIDTH {
            self.buf.str[row][col] = blank;
        }
    }

}

impl fmt::Write for Logger {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_str(s);
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



#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::kernel_components::vga_buffer::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => (print!("\n"));
    ($fmt:expr) => (print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => (print!(concat!($fmt, "\n"), $($arg)*));
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    LOGGER.lock().write_fmt(args).unwrap();
}