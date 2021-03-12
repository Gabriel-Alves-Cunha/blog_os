#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)] // Enable copy semantics for the type and make it printable and comparable.
#[repr(u8)]
pub enum Color {
  Black = 0,
  Blue = 1,
  Green = 2,
  Cyan = 3,
  Red = 4,
  Magenta = 5,
  Brown = 6,
  LightGray = 7,
  DarkGrey = 8,
  LightBlue = 9,
  LightGreen = 10,
  LightCyan = 11,
  LightRed = 12,
  Pink = 13,
  Yellow = 14,
  White = 15,
}

///////////////////////////////////////////////////////////

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)] // Ensure that it has the same memory layout as its single field.
struct ColorCode(u8);

impl ColorCode {
  fn new(foreground: Color, background: Color) -> ColorCode {
    ColorCode((background as u8) << 4 | (foreground as u8))
  }
}

///////////////////////////////////////////////////////////

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)] // Since the field ordering in default structs is undefined in Rust, we need the repr(C) attribute. It guarantees that the struct's fields are laid out exactly like in a C struct and thus guarantees the correct field ordering.
struct ScreenChar {
  ascii_character: u8,
  color_code: ColorCode,
}

const BUFFER_HEIGHT: usize = 25;
const BUFFER_WIDTH: usize = 80;

use volatile::Volatile;
// A structure representing the VGA text buffer.
#[repr(transparent)]
struct Buffer {
  chars: [[Volatile<ScreenChar>; BUFFER_WIDTH]; BUFFER_HEIGHT],
}

// To actually write to screen, we now create a writer type:
pub struct Writer {
  column_position: usize,
  color_code: ColorCode,
  buffer: &'static mut Buffer, //The 'static lifetime specifies that the reference is valid for the whole program run time (which is true for the VGA text buffer).
}

// Printing: now we can use the Writer to modify the buffer's characters. First we create a method to write a single ASCII byte:
impl Writer {
  pub fn write_byte(&mut self, byte: u8) {
    match byte {
      b'\n' => self.new_line(),
      byte => {
        if self.column_position >= BUFFER_WIDTH {
          // Check if the current line is full.
          self.new_line();
        }

        let row = BUFFER_HEIGHT - 1;
        let col = self.column_position;
        let color_code = self.color_code;

        self.buffer.chars[row][col].write(ScreenChar {
          ascii_character: byte,
          color_code,
        }); // Instead of a normal assignment using =, we're now using the write method. This guarantees that the compiler will never optimize away this write.

        self.column_position += 1;
      }
    }
  }

  fn new_line(&mut self) {
    for row in 1..BUFFER_HEIGHT {
      for col in 0..BUFFER_WIDTH {
        let character = self.buffer.chars[row][col].read();
        self.buffer.chars[row - 1][col].write(character);
      }
    }

    self.clear_row(BUFFER_HEIGHT - 1);
    self.column_position = 0;
  }

  // to clear_row clears a row by overwriting all of its characters with a space character.
  fn clear_row(&mut self, row: usize) {
    let blank = ScreenChar {
      ascii_character: b' ',
      color_code: self.color_code,
    };

    for col in 0..BUFFER_WIDTH {
      self.buffer.chars[row][col].write(blank);
    }
  }

  // To print whole strings, we can convert them to bytes and print them one-by-one:
  pub fn write_string(&mut self, s: &str) {
    for byte in s.bytes() {
      match byte {
        // Printable ASCII byte or new line:
        0x20..=0x7e | b'\n' => self.write_byte(byte),

        // The VGA text buffer only supports ASCII and the additional bytes of code page 437. Rust strings are UTF-8 by default, so they might contain bytes that are not supported by the VGA text buffer.
        // Not part of printable ASCII range
        _ => self.write_byte(0xfe), // 0xfe = ■
      }
    }
  }
}

use core::fmt;
impl fmt::Write for Writer {
  fn write_str(&mut self, s: &str) -> fmt::Result {
    self.write_string(s);
    Ok(())
  }
}

// pub fn test_print() {
//   use core::fmt::Write;

//   let mut writer = Writer {
//     column_position: 0,
//     color_code: ColorCode::new(Color::Yellow, Color::Black),
//     buffer: unsafe { &mut *(0xb8000 as *mut Buffer) }, // First, we cast the integer 0xb8000 as a mutable raw pointer. Then we convert it to a mutable reference by dereferencing it (through *) and immediately borrowing it again (through &mut). This conversion requires an unsafe block, since the compiler can't guarantee that the raw pointer is valid.
//   };

//   writer.write_byte(b'H'); // The b prefix creates a byte literal, which represents an ASCII character
//   writer.write_string("ello ");
//   writer.write_string("Wórld!");
//   write!(writer, "The numbers are {} and {}", 42, 1.0 / 3.0).unwrap(); // The write! call returns a Result which causes a warning if not used, so we call the unwrap function on it, which panics if an error occurs. This isn't a problem in our case, since writes to the VGA buffer never fail.
// }

// To provide a global writer that can be used as an interface from other modules without carrying a Writer instance around, we try to create a static WRITER:
use lazy_static::lazy_static;
use spin::Mutex;

lazy_static! {
  pub static ref WRITER: Mutex<Writer> = Mutex::new(Writer {
    column_position: 0,
    color_code: ColorCode::new(Color::Yellow, Color::Black),
    buffer: unsafe { &mut *(0xb8000 as *mut Buffer) }
  });
}

#[macro_export]
macro_rules! print {
  ($($arg:tt)*) => ($crate::vga_buffer::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
  () => ($crate::print!("\n"));
  ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[doc(hidden)] // Since the macros need to be able to call _print from outside of the module, the function needs to be public. However, since we consider this a private implementation detail, we add the doc(hidden) attribute to hide it from the generated documentation. Prints the given formatted string to the VGA text buffer through the global `WRITER` instance.
pub fn _print(args: fmt::Arguments) {
  use core::fmt::Write;
  use x86_64::instructions::interrupts;

  interrupts::without_interrupts(|| {
    WRITER.lock().write_fmt(args).unwrap();
  });
}

///////////////////////////////////////////////////////////

#[test_case]
fn test_println_simple() {
  println!("test_println_simple output");
}

#[test_case]
fn test_println_many() {
  for _ in 0..200 {
    println!("test_println_many output");
  }
}

#[test_case]
fn test_println_output() {
  use core::fmt::Write;
  use x86_64::instructions::interrupts;

  let s = "Some test string that fits on a single line";

  interrupts::without_interrupts(|| {
    let mut writer = WRITER.lock();
    writeln!(writer, "\n{}", s).expect("writeln failed");

    for (i, c) in s.chars().enumerate() {
      let screen_char = writer.buffer.chars[BUFFER_HEIGHT - 2][i].read();
      assert_eq!(char::from(screen_char.ascii_character), c);
    }
  });
}
