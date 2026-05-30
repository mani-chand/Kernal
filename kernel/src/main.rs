#![no_std]
#![no_main]

use core::panic::PanicInfo;
use bootloader_api::{entry_point, BootInfo};

// This macro creates the real entry point and maps everything correctly
entry_point!(kernel_main);

fn get_char_bitmap(c: char) -> [u8; 8] {
    match c {
        'H' => [0x66, 0x66, 0x66, 0x7e, 0x66, 0x66, 0x66, 0x00],
        'e' => [0x00, 0x3c, 0x66, 0x7e, 0x60, 0x66, 0x3c, 0x00],
        'l' => [0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x3c, 0x00],
        'o' => [0x00, 0x3c, 0x66, 0x66, 0x66, 0x66, 0x3c, 0x00],
        'W' => [0x66, 0x66, 0x66, 0x6a, 0x76, 0x76, 0x66, 0x00],
        'r' => [0x00, 0x78, 0x66, 0x60, 0x60, 0x60, 0x60, 0x00],
        'd' => [0x06, 0x06, 0x3e, 0x66, 0x66, 0x66, 0x3e, 0x00],
        ' ' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
        _   => [0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
    }
}

fn draw_char(
    buffer: &mut [u8],
    stride: usize,
    bytes_per_pixel: usize,
    r_idx: usize,
    g_idx: usize,
    b_idx: usize,
    x_pos: usize,
    y_pos: usize,
    c: char,
    scale: usize,
) {
    let bitmap = get_char_bitmap(c);
    for row in 0..8 {
        let byte = bitmap[row];
        for col in 0..8 {
            let bit_set = (byte & (0x80 >> col)) != 0;
            if bit_set {
                for dy in 0..scale {
                    for dx in 0..scale {
                        let px = x_pos + col * scale + dx;
                        let py = y_pos + row * scale + dy;
                        let pixel_offset = (py * stride + px) * bytes_per_pixel;
                        buffer[pixel_offset + r_idx] = 0xff;
                        buffer[pixel_offset + g_idx] = 0xff;
                        buffer[pixel_offset + b_idx] = 0xff;
                    }
                }
            }
        }
    }
}

fn draw_string(
    buffer: &mut [u8],
    stride: usize,
    bytes_per_pixel: usize,
    r_idx: usize,
    g_idx: usize,
    b_idx: usize,
    start_x: usize,
    start_y: usize,
    s: &str,
    scale: usize,
) {
    let mut current_x = start_x;
    for c in s.chars() {
        draw_char(buffer, stride, bytes_per_pixel, r_idx, g_idx, b_idx, current_x, start_y, c, scale);
        current_x += 8 * scale + 2 * scale;
    }
}

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    if let Some(framebuffer) = boot_info.framebuffer.as_mut() {
        let info = framebuffer.info();
        let buffer = framebuffer.buffer_mut();
        
        let bytes_per_pixel = info.bytes_per_pixel as usize;
        let stride = info.stride as usize;
        let width = info.width as usize;
        let height = info.height as usize;

        // Detect RGB vs BGR layout
        let (r_idx, g_idx, b_idx) = match info.pixel_format {
            bootloader_api::info::PixelFormat::Rgb => (0, 1, 2),
            bootloader_api::info::PixelFormat::Bgr => (2, 1, 0),
            _ => (0, 1, 2),
        };

        // Draw a dark background
        for y in 0..height {
            for x in 0..width {
                let pixel_offset = (y * stride + x) * bytes_per_pixel;
                if bytes_per_pixel >= 3 {
                    buffer[pixel_offset + r_idx] = 0x20;
                    buffer[pixel_offset + g_idx] = 0x20;
                    buffer[pixel_offset + b_idx] = 0x2e;
                } else {
                    buffer[pixel_offset] = 0x20;
                }
            }
        }

        // Draw "Hello World" in the center (string length is 11, width approx 11 * 10 * scale)
        let scale = 4;
        let text = "Hello World";
        let text_width = text.len() * (8 * scale + 2 * scale);
        let start_x = if width > text_width { (width - text_width) / 2 } else { 10 };
        let start_y = if height > 8 * scale { (height - 8 * scale) / 2 } else { 10 };

        draw_string(buffer, stride, bytes_per_pixel, r_idx, g_idx, b_idx, start_x, start_y, text, scale);
    }

    loop {}
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
