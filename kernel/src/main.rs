#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

use core::panic::PanicInfo;
use core::fmt;
use bootloader_api::{ entry_point, BootInfo };
use spin::Mutex;
mod interrupts;
mod speaker;
mod time;

// Global screen writer protected by a spinlock Mutex.
// Initially, it's empty (None) until we initialize it in kernel_main.
pub static WRITER: Mutex<Option<FrameBufferWriter>> = Mutex::new(None);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    Black = 0,
    Blue,
    Green,
    Cyan,
    Red,
    Magenta,
    Brown,
    LightGray,
    DarkGray,
    LightBlue,
    LightGreen,
    LightCyan,
    LightRed,
    Pink,
    Yellow,
    White,
}

impl Color {
    /// Returns the (Red, Green, Blue) byte values for each color
    pub fn get_rgb(self) -> (u8, u8, u8) {
        match self {
            Color::Black => (0x00, 0x00, 0x00),
            Color::Blue => (0x00, 0x00, 0xaa),
            Color::Green => (0x00, 0xaa, 0x00),
            Color::Cyan => (0x00, 0xaa, 0xaa),
            Color::Red => (0xaa, 0x00, 0x00),
            Color::Magenta => (0xaa, 0x00, 0xaa),
            Color::Brown => (0xaa, 0x55, 0x00),
            Color::LightGray => (0xaa, 0xaa, 0xaa),
            Color::DarkGray => (0x55, 0x55, 0x55),
            Color::LightBlue => (0x55, 0x55, 0xff),
            Color::LightGreen => (0x00, 0xff, 0x00), // Pure bright green for hacker theme
            Color::LightCyan => (0x55, 0xff, 0xff),
            Color::LightRed => (0xff, 0x55, 0x55),
            Color::Pink => (0xff, 0x55, 0xff),
            Color::Yellow => (0xff, 0xff, 0x55),
            Color::White => (0xff, 0xff, 0xff),
        }
    }
}

// 8x8 bitmap font data for drawing characters
// A basic 8x8 font bitmap for standard printable ASCII characters 32 to 126
const FONT_8X8: [[u8; 8]; 95] = [
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], // 32: ' ' (Space)
    [0x18, 0x3c, 0x3c, 0x18, 0x18, 0x00, 0x18, 0x00], // 33: '!'
    [0x36, 0x6d, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], // 34: '"'
    [0x36, 0x36, 0x7f, 0x36, 0x7f, 0x36, 0x36, 0x00], // 35: '#'
    [0x1e, 0x30, 0x1c, 0x06, 0x3c, 0x18, 0x00, 0x00], // 36: '$'
    [0x3a, 0x6e, 0x08, 0x10, 0x20, 0x76, 0x5c, 0x00], // 37: '%'
    [0x3c, 0x66, 0x3c, 0x38, 0x67, 0x66, 0x3f, 0x00], // 38: '&'
    [0x18, 0x3c, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], // 39: '\''
    [0x0c, 0x18, 0x30, 0x30, 0x30, 0x18, 0x0c, 0x00], // 40: '('
    [0x30, 0x18, 0x0c, 0x0c, 0x0c, 0x18, 0x30, 0x00], // 41: ')'
    [0x00, 0x66, 0x3c, 0xff, 0x3c, 0x66, 0x00, 0x00], // 42: '*'
    [0x00, 0x18, 0x18, 0x7e, 0x18, 0x18, 0x00, 0x00], // 43: '+'
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x18, 0x18, 0x30], // 44: ','
    [0x00, 0x00, 0x00, 0x7e, 0x00, 0x00, 0x00, 0x00], // 45: '-'
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x18, 0x18, 0x00], // 46: '.'
    [0x03, 0x06, 0x0c, 0x18, 0x30, 0x60, 0x40, 0x00], // 47: '/'
    [0x3e, 0x67, 0x6f, 0x7b, 0x73, 0x63, 0x3e, 0x00], // 48: '0'
    [0x18, 0x38, 0x18, 0x18, 0x18, 0x18, 0x7e, 0x00], // 49: '1'
    [0x3e, 0x63, 0x07, 0x0e, 0x1c, 0x30, 0x7f, 0x00], // 50: '2'
    [0x7e, 0x03, 0x03, 0x3e, 0x03, 0x03, 0x7e, 0x00], // 51: '3'
    [0x0c, 0x1c, 0x3c, 0x6c, 0x7f, 0x0c, 0x0c, 0x00], // 52: '4'
    [0x7f, 0x60, 0x7e, 0x03, 0x03, 0x63, 0x3e, 0x00], // 53: '5'
    [0x1c, 0x30, 0x60, 0x7e, 0x63, 0x63, 0x3e, 0x00], // 54: '6'
    [0x7f, 0x03, 0x06, 0x0c, 0x18, 0x18, 0x18, 0x00], // 55: '7'
    [0x3e, 0x63, 0x63, 0x3e, 0x63, 0x63, 0x3e, 0x00], // 56: '8'
    [0x3e, 0x63, 0x63, 0x7f, 0x03, 0x06, 0x3c, 0x00], // 57: '9'
    [0x00, 0x18, 0x18, 0x00, 0x00, 0x18, 0x18, 0x00], // 58: ':'
    [0x00, 0x18, 0x18, 0x00, 0x00, 0x18, 0x18, 0x30], // 59: ';'
    [0x0c, 0x18, 0x30, 0x60, 0x30, 0x18, 0x0c, 0x00], // 60: '<'
    [0x00, 0x00, 0x7e, 0x00, 0x7e, 0x00, 0x00, 0x00], // 61: '='
    [0x30, 0x18, 0x0c, 0x06, 0x0c, 0x18, 0x30, 0x00], // 62: '>'
    [0x3e, 0x63, 0x07, 0x0e, 0x18, 0x00, 0x18, 0x00], // 63: '?'
    [0x3e, 0x63, 0x6f, 0x6f, 0x6e, 0x60, 0x3e, 0x00], // 64: '@'
    [0x18, 0x3c, 0x66, 0x7e, 0x66, 0x66, 0x66, 0x00], // 65: 'A'
    [0x7c, 0x66, 0x66, 0x7c, 0x66, 0x66, 0x7c, 0x00], // 66: 'B'
    [0x3e, 0x63, 0x60, 0x60, 0x60, 0x63, 0x3e, 0x00], // 67: 'C'
    [0x78, 0x6c, 0x66, 0x66, 0x66, 0x6c, 0x78, 0x00], // 68: 'D'
    [0x7f, 0x60, 0x60, 0x7c, 0x60, 0x60, 0x7f, 0x00], // 69: 'E'
    [0x7f, 0x60, 0x60, 0x7c, 0x60, 0x60, 0x60, 0x00], // 70: 'F'
    [0x3e, 0x63, 0x60, 0x6f, 0x63, 0x63, 0x3d, 0x00], // 71: 'G'
    [0x66, 0x66, 0x66, 0x7e, 0x66, 0x66, 0x66, 0x00], // 72: 'H'
    [0x3e, 0x0c, 0x0c, 0x0c, 0x0c, 0x0c, 0x3e, 0x00], // 73: 'I'
    [0x07, 0x03, 0x03, 0x03, 0x03, 0x63, 0x3e, 0x00], // 74: 'J'
    [0x66, 0x6c, 0xd8, 0xf0, 0xd8, 0x6c, 0x66, 0x00], // 75: 'K'
    [0x60, 0x60, 0x60, 0x60, 0x60, 0x60, 0x7f, 0x00], // 76: 'L'
    [0x63, 0x77, 0x7f, 0x6b, 0x63, 0x63, 0x63, 0x00], // 77: 'M'
    [0x63, 0x73, 0x7b, 0x6f, 0x67, 0x63, 0x63, 0x00], // 78: 'N'
    [0x3e, 0x63, 0x63, 0x63, 0x63, 0x63, 0x3e, 0x00], // 79: 'O'
    [0x7c, 0x66, 0x66, 0x7c, 0x60, 0x60, 0x60, 0x00], // 80: 'P'
    [0x3e, 0x63, 0x63, 0x63, 0x6b, 0x66, 0x3d, 0x00], // 81: 'Q'
    [0x7c, 0x66, 0x66, 0x7c, 0x6c, 0x66, 0x66, 0x00], // 82: 'R'
    [0x3e, 0x63, 0x30, 0x1c, 0x06, 0x63, 0x3e, 0x00], // 83: 'S'
    [0x7f, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x00], // 84: 'T'
    [0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x3e, 0x00], // 85: 'U'
    [0x66, 0x66, 0x66, 0x66, 0x66, 0x3c, 0x18, 0x00], // 86: 'V'
    [0x63, 0x63, 0x63, 0x6b, 0x7f, 0x77, 0x63, 0x00], // 87: 'W'
    [0x63, 0x63, 0x36, 0x1c, 0x36, 0x63, 0x63, 0x00], // 88: 'X'
    [0x66, 0x66, 0x66, 0x3c, 0x18, 0x18, 0x18, 0x00], // 89: 'Y'
    [0x7f, 0x06, 0x0c, 0x18, 0x30, 0x60, 0x7f, 0x00], // 90: 'Z'
    [0x3e, 0x30, 0x30, 0x30, 0x30, 0x30, 0x3e, 0x00], // 91: '['
    [0x40, 0x30, 0x18, 0x0c, 0x06, 0x03, 0x01, 0x00], // 92: '\\'
    [0x3e, 0x0c, 0x0c, 0x0c, 0x0c, 0x0c, 0x3e, 0x00], // 93: ']'
    [0x18, 0x3c, 0x66, 0x00, 0x00, 0x00, 0x00, 0x00], // 94: '^'
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0x00], // 95: '_'
    [0x18, 0x0c, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], // 96: '`'
    [0x00, 0x00, 0x3c, 0x06, 0x3e, 0x66, 0x3b, 0x00], // 97: 'a'
    [0x60, 0x60, 0x7c, 0x66, 0x66, 0x66, 0x7c, 0x00], // 98: 'b'
    [0x00, 0x00, 0x3c, 0x60, 0x60, 0x63, 0x3e, 0x00], // 99: 'c'
    [0x06, 0x06, 0x3e, 0x66, 0x66, 0x66, 0x3e, 0x00], // 100: 'd'
    [0x00, 0x00, 0x3e, 0x66, 0x7e, 0x60, 0x3c, 0x00], // 101: 'e'
    [0x1c, 0x30, 0x7c, 0x30, 0x30, 0x30, 0x30, 0x00], // 102: 'f'
    [0x00, 0x00, 0x3e, 0x66, 0x66, 0x3e, 0x06, 0x3c], // 103: 'g'
    [0x60, 0x60, 0x7c, 0x66, 0x66, 0x66, 0x66, 0x00], // 104: 'h'
    [0x18, 0x00, 0x38, 0x18, 0x18, 0x18, 0x3c, 0x00], // 105: 'i'
    [0x06, 0x00, 0x0e, 0x06, 0x06, 0x66, 0x3c, 0x00], // 106: 'j'
    [0x60, 0x60, 0x66, 0x6c, 0x78, 0x6c, 0x66, 0x00], // 107: 'k'
    [0x38, 0x18, 0x18, 0x18, 0x18, 0x18, 0x3c, 0x00], // 108: 'l'
    [0x00, 0x00, 0x66, 0x7f, 0x7f, 0x6b, 0x63, 0x00], // 109: 'm'
    [0x00, 0x00, 0x7c, 0x66, 0x66, 0x66, 0x66, 0x00], // 110: 'n'
    [0x00, 0x00, 0x3e, 0x66, 0x66, 0x66, 0x3e, 0x00], // 111: 'o'
    [0x00, 0x00, 0x7c, 0x66, 0x66, 0x7c, 0x60, 0x60], // 112: 'p'
    [0x00, 0x00, 0x3e, 0x66, 0x66, 0x3e, 0x06, 0x07], // 113: 'q'
    [0x00, 0x00, 0x7c, 0x66, 0x60, 0x60, 0x60, 0x00], // 114: 'r'
    [0x00, 0x00, 0x3e, 0x60, 0x3c, 0x03, 0x7e, 0x00], // 115: 's'
    [0x18, 0x18, 0x7c, 0x18, 0x18, 0x18, 0x0c, 0x00], // 116: 't'
    [0x00, 0x00, 0x66, 0x66, 0x66, 0x66, 0x3b, 0x00], // 117: 'u'
    [0x00, 0x00, 0x66, 0x66, 0x66, 0x3c, 0x18, 0x00], // 118: 'v'
    [0x00, 0x00, 0x63, 0x6b, 0x6b, 0x7f, 0x36, 0x00], // 119: 'w'
    [0x00, 0x00, 0x63, 0x36, 0x1c, 0x36, 0x63, 0x00], // 120: 'x'
    [0x00, 0x00, 0x66, 0x66, 0x66, 0x3e, 0x06, 0x3c], // 121: 'y'
    [0x00, 0x00, 0x7f, 0x0c, 0x18, 0x30, 0x7f, 0x00], // 122: 'z'
    [0x0e, 0x18, 0x18, 0x70, 0x18, 0x18, 0x0e, 0x00], // 123: '{'
    [0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x00], // 124: '|'
    [0x70, 0x18, 0x18, 0x0e, 0x18, 0x18, 0x70, 0x00], // 125: '}'
    [0x3b, 0x6e, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], // 126: '~'
];

fn get_char_bitmap(c: char) -> [u8; 8] {
    let code = c as usize;
    // Standard printable ASCII characters range from 32 (space) to 126 (~)
    if (32..=126).contains(&code) {
        FONT_8X8[code - 32]
    } else {
        // Return a blank space (no pixels) for unsupported characters
        [0x00; 8]
    }
}

pub struct FrameBufferWriter {
    buffer: &'static mut [u8],
    info: bootloader_api::info::FrameBufferInfo,
    x_pos: usize,
    y_pos: usize,
    r_idx: usize,
    g_idx: usize,
    b_idx: usize,
    // Add these fields to track line history:
    line_end_positions: [usize; 256],
    current_line_index: usize,
    current_color: Color,
}

impl FrameBufferWriter {
    pub fn new(buffer: &'static mut [u8], info: bootloader_api::info::FrameBufferInfo) -> Self {
        // Detect RGB vs BGR layout
        let (r_idx, g_idx, b_idx) = match info.pixel_format {
            bootloader_api::info::PixelFormat::Rgb => (0, 1, 2),
            bootloader_api::info::PixelFormat::Bgr => (2, 1, 0),
            _ => (0, 1, 2),
        };

        let mut writer = Self {
            buffer,
            info,
            x_pos: 10,
            y_pos: 10,
            r_idx,
            g_idx,
            b_idx,
            // Initialize history:
            line_end_positions: [0; 256],
            current_line_index: 0,
            current_color: Color::LightGreen,
        };
        writer.clear();
        writer
    }

    // Add this helper method inside the impl FrameBufferWriter block
    pub fn set_color(&mut self, color: Color) {
        self.current_color = color;
    }

    // --- ADD THIS METHOD ---
    pub fn erase_last_char(&mut self) {
        let scale = 2;
        let char_width = 8 * scale + 2;

        // 1. Erase the cursor at the old position
        self.draw_cursor(false);

        // If we are at the start of a line, check if we can jump back to the previous line
        if self.x_pos == 10 && self.current_line_index > 0 {
            self.current_line_index -= 1;
            self.y_pos -= 20; // Move up 20 pixels
            self.x_pos = self.line_end_positions[self.current_line_index]; // Restore previous line's X ending position;
        }

        // Only erase if we are not at the very start of the first line
        if self.x_pos > 10 {
            // Move the cursor back by one character width
            self.x_pos -= char_width;

            let bytes_per_pixel = self.info.bytes_per_pixel as usize;
            let stride = self.info.stride as usize;
            let char_height = 8 * scale;

            // Draw a solid dark blue block over the character to erase it
            for y in 0..char_height {
                for x in 0..char_width {
                    let px = self.x_pos + x;
                    let py = self.y_pos + y;
                    let pixel_offset = (py * stride + px) * bytes_per_pixel;
                    if pixel_offset + 2 < self.buffer.len() {
                        if bytes_per_pixel >= 3 {
                            // Overwrite with pure black (RGB: 0, 0, 0)
                            self.buffer[pixel_offset + self.r_idx] = 0x00;
                            self.buffer[pixel_offset + self.g_idx] = 0x00;
                            self.buffer[pixel_offset + self.b_idx] = 0x00;
                        } else {
                            self.buffer[pixel_offset] = 0x20;
                        }
                    }
                }
            }
        }

        // 2. Draw the cursor at the new position (after deleting/backtracking)
        self.draw_cursor(true);
    }
    pub fn draw_cursor(&mut self, show: bool) {
        let scale = 2;
        let width = 8 * scale;
        let height = 2 * scale;

        // White cursor when showing, Black when hiding
        let r_color = if show { 0xff } else { 0x00 };
        let g_color = if show { 0xff } else { 0x00 };
        let b_color = if show { 0xff } else { 0x00 };

        let bytes_per_pixel = self.info.bytes_per_pixel as usize;
        let stride = self.info.stride as usize;

        let start_y = self.y_pos + 8 * scale - height;

        for y in 0..height {
            for x in 0..width {
                let px = self.x_pos + x;
                let py = start_y + y;
                let pixel_offset = (py * stride + px) * bytes_per_pixel;
                if pixel_offset + 2 < self.buffer.len() {
                    if bytes_per_pixel >= 3 {
                        self.buffer[pixel_offset + self.r_idx] = r_color;
                        self.buffer[pixel_offset + self.g_idx] = g_color;
                        self.buffer[pixel_offset + self.b_idx] = b_color;
                    } else {
                        self.buffer[pixel_offset] = r_color;
                    }
                }
            }
        }
    }

    /// Clears the screen with a dark blue color
    pub fn clear(&mut self) {
        let bytes_per_pixel = self.info.bytes_per_pixel as usize;
        let stride = self.info.stride as usize;
        let width = self.info.width as usize;
        let height = self.info.height as usize;

        for y in 0..height {
            for x in 0..width {
                let pixel_offset = (y * stride + x) * bytes_per_pixel;
                if bytes_per_pixel >= 3 {
                    // Set background to pure black (RGB: 0, 0, 0)
                    self.buffer[pixel_offset + self.r_idx] = 0x00;
                    self.buffer[pixel_offset + self.g_idx] = 0x00;
                    self.buffer[pixel_offset + self.b_idx] = 0x00;
                } else {
                    self.buffer[pixel_offset] = 0x20;
                }
            }
        }
        self.x_pos = 10;
        self.y_pos = 10;
        self.current_line_index = 0; // Reset line history index
        self.draw_cursor(true); // <-- Draw cursor at top-left
    }

    /// Draws a single character to the framebuffer
    pub fn write_char(&mut self, c: char) {
        let bytes_per_pixel = self.info.bytes_per_pixel as usize;
        let stride = self.info.stride as usize;
        let width = self.info.width as usize;
        let height = self.info.height as usize;
        let scale = 2; // Character scaling size

        // 1. Erase the cursor at the old position
        self.draw_cursor(false);

        // Handle newlines
        if c == '\n' {
            if self.current_line_index < 255 {
                self.line_end_positions[self.current_line_index] = self.x_pos;
                self.current_line_index += 1;
            }
            self.newline();
            // 2. Draw the cursor at the start of the new line
            self.draw_cursor(true);
            return;
        }

        // Auto wrap line if we reach the screen edge
        let char_width = 8 * scale + 2;

        // Handle spaces directly to avoid drawing boxes
        if c == ' ' {
            self.x_pos += char_width;
            // 2. Draw the cursor at the new space position
            self.draw_cursor(true);
            return;
        }

        if self.x_pos + char_width >= width {
            if self.current_line_index < 255 {
                self.line_end_positions[self.current_line_index] = self.x_pos;
                self.current_line_index += 1;
            }
            self.newline();
        }

        // If we go off the bottom of the screen, reset to top (scrolling can be added later)
        let char_height = 8 * scale;
        if self.y_pos + char_height >= height {
            self.clear();
        }

        let bitmap = get_char_bitmap(c);
        let (r, g, b) = self.current_color.get_rgb();
        for row in 0..8 {
            let byte = bitmap[row];
            for col in 0..8 {
                let bit_set = (byte & (0x80 >> col)) != 0;
                if bit_set {
                    for dy in 0..scale {
                        for dx in 0..scale {
                            let px = self.x_pos + col * scale + dx;
                            let py = self.y_pos + row * scale + dy;
                            let pixel_offset = (py * stride + px) * bytes_per_pixel;
                            if pixel_offset + 2 < self.buffer.len() {
                                // Draw character in bright green (RGB: 0, 255, 0)
                                self.buffer[pixel_offset + self.r_idx] = r;
                                self.buffer[pixel_offset + self.g_idx] = g;
                                self.buffer[pixel_offset + self.b_idx] = b;
                            }
                        }
                    }
                }
            }
        }
        self.x_pos += char_width;

        // 2. Draw the cursor at the new position
        self.draw_cursor(true);
    }

    fn newline(&mut self) {
        self.x_pos = 10;
        self.y_pos += 20; // Move down 20 pixels
    }
}
// Implement core::fmt::Write so our writer supports print formatting macros (like write!)
impl fmt::Write for FrameBufferWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            self.write_char(c);
        }
        Ok(())
    }
}
// Defining our custom print and println macros
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    // Lock the global writer and write the formatted string to it
    if let Some(writer) = WRITER.lock().as_mut() {
        writer.write_fmt(args).unwrap();
    }
}

pub fn erase_char() {
    if let Some(writer) = WRITER.lock().as_mut() {
        writer.erase_last_char();
    }
}

#[macro_export]
macro_rules! print_color {
    (
        $color:expr,
        $($arg:tt)*
    ) => ($crate::_print_color($color, format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println_color {
    ($color:expr) => ($crate::print_color!($color, "\n"));
    (
        $color:expr,
        $($arg:tt)*
    ) => ($crate::print_color!($color, "{}\n", format_args!($($arg)*)));
}

#[doc(hidden)]
pub fn _print_color(color: Color, args: fmt::Arguments) {
    use core::fmt::Write;
    if let Some(writer) = WRITER.lock().as_mut() {
        let prev_color = writer.current_color;
        writer.set_color(color);
        writer.write_fmt(args).unwrap();
        writer.set_color(prev_color); // Restore previous color state
    }
}

// A thread-safe global command buffer of size 80
pub static CMD_BUFFER: Mutex<CommandBuf> = Mutex::new(CommandBuf {
    buf: [0; 80],
    len: 0,
});

pub struct CommandBuf {
    pub buf: [u8; 80],
    pub len: usize,
}

impl CommandBuf {
    pub fn push(&mut self, c: u8) -> bool {
        if self.len < self.buf.len() {
            self.buf[self.len] = c;
            self.len += 1;
            true
        } else {
            false
        }
    }

    pub fn pop(&mut self) -> bool {
        if self.len > 0 {
            self.len -= 1;
            true
        } else {
            false
        }
    }

    pub fn clear(&mut self) {
        self.len = 0;
    }
}

pub fn print_prompt() {
    print_color!(Color::LightCyan, "arch-rust > "); // Cyan prompt
}

pub fn interpret_command(cmd_str: &str) {
    let trimmed = cmd_str.trim();
    if trimmed.is_empty() {
        return;
    }

    let mut parts = trimmed.splitn(2, ' ');
    let cmd = parts.next().unwrap_or("");
    let args = parts.next().unwrap_or("");

    match cmd {
        "help" => {
            println_color!(Color::Yellow, "Available commands:");
            println!("  help  - Show this help menu");
            println!("  clear - Clear the screen");
            println!("  about - About this OS");
            println!("  echo  - Repeat text back (e.g., 'echo hello')");
            println!("  panic - Force trigger a CPU panic");
        }
        "clear" => {
            if let Some(writer) = WRITER.lock().as_mut() {
                writer.clear();
            }
        }
        "about" => {
            println_color!(Color::LightGreen, "Arch-Rust OS v0.1.0");
            println!("A minimal 64-bit freestanding OS written in Rust.");
        }
        "beep" => {
            let freq_str = args.trim();
            let freq = if freq_str.is_empty() {
                440
            } else {
                freq_str.parse::<u32>().unwrap_or(440)
            };

            println_color!(Color::LightCyan, "Beeping at {} Hz...", freq);
            speaker::beep(freq, 200); // <-- Change this to pass 200ms
        }
        // ------------------------------
        "echo" => {
            println_color!(Color::White, "{}", args);
        }
        "panic" => {
            panic!("User triggered manual kernel panic!");
        }
        _ => {
            // Print error messages in red!
            println_color!(Color::LightRed, "Unknown command: '{}'. Type 'help' for options.", cmd);
        }
    }
}

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    if let Some(framebuffer) = boot_info.framebuffer.as_mut() {
        let info = framebuffer.info();
        let buffer = framebuffer.buffer_mut();

        // Initialize the global WRITER
        *WRITER.lock() = Some(FrameBufferWriter::new(buffer, info));
    }

    interrupts::init_idt();
    // --- ADD THIS TIMER INITIALIZATION ---
    println!("Initializing PIT system timer (1ms ticks)...");
    time::init_pit();
    unsafe {
        interrupts::PICS.lock().initialize();
    }

    x86_64::instructions::interrupts::enable(); // Tells the CPU to start listening to hardware interrupts
    println!("Interrupts enabled! Try typing on your keyboard...");

    println!("Arch-Rust OS Kernel v0.1.0");
    println!("--------------------------------");
    println!("Type 'help' to see available commands.");
    println!();
    print_prompt(); // Draw the starting prompt line

    loop {
        x86_64::instructions::hlt();
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}
