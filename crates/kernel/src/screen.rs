use font8x8::legacy::{BASIC_LEGACY, NOTHING_TO_DISPLAY};
use pomelo_common::GraphicConfig;

pub struct Color {
    r: u8,
    g: u8,
    b: u8,
}
#[allow(unused)]
impl Color {
    pub const BLACK: Color = Color { r: 0, g: 0, b: 0 };
    pub const WHITE: Color = Color {
        r: 255,
        g: 255,
        b: 255,
    };
    pub const RED: Color = Color { r: 255, g: 0, b: 0 };
    pub const GREEN: Color = Color { r: 0, g: 255, b: 0 };
    pub const BLUE: Color = Color { r: 0, g: 0, b: 255 };
}

pub struct Screen {
    buffer: &'static mut [u8],
    r_offset: u8,
    g_offset: u8,
    b_offset: u8,
    horisontal_resolution: usize,
    vertical_resolution: usize,
    stride: usize,
}

impl Screen {
    pub fn from(config: &GraphicConfig) -> Self {
        Self {
            buffer: unsafe {
                core::slice::from_raw_parts_mut(config.frame_buffer_base, config.frame_buffer_size)
            },
            r_offset: config.pixel_format.r_offset(),
            g_offset: config.pixel_format.g_offset(),
            b_offset: config.pixel_format.b_offset(),
            horisontal_resolution: config.horisontal_resolution,
            vertical_resolution: config.vertical_resolution,
            stride: config.stride,
        }
    }

    pub fn width(&self) -> usize {
        self.horisontal_resolution
    }

    pub fn height(&self) -> usize {
        self.vertical_resolution
    }

    pub fn write(&mut self, x: usize, y: usize, color: &Color) {
        let pos = 4 * (self.stride * y + x);
        self.buffer[pos + self.r_offset as usize] = color.r;
        self.buffer[pos + self.g_offset as usize] = color.g;
        self.buffer[pos + self.b_offset as usize] = color.b;
    }

    pub fn write_char(&mut self, x: usize, y: usize, color: &Color, c: u8) {
        let glyph = BASIC_LEGACY.get(c as usize).unwrap_or(&NOTHING_TO_DISPLAY);
        for (dy, row) in glyph
            .iter()
            .flat_map(|r| core::iter::repeat(*r).take(2))
            .enumerate()
        {
            for dx in 0..8 {
                if ((row >> dx) & 1) != 0 {
                    self.write(x + dx, y + dy, color);
                }
            }
        }
    }

    pub fn write_string(&mut self, x: usize, y: usize, color: &Color, s: &str) {
        for (i, c) in s.bytes().enumerate() {
            self.write_char(x + i * 8, y, color, c);
        }
    }

    pub fn write_fmt(
        &mut self,
        x: usize,
        y: usize,
        color: &Color,
        buffer: &mut [u8],
        args: core::fmt::Arguments,
    ) -> Result<(), core::fmt::Error> {
        struct WriteBuffer<'a> {
            buffer: &'a mut [u8],
            used: usize,
        }
        impl<'a> core::fmt::Write for WriteBuffer<'a> {
            fn write_str(&mut self, s: &str) -> core::fmt::Result {
                let to_write = s.as_bytes();
                if self.used + to_write.len() > self.buffer.len() {
                    Err(core::fmt::Error)
                } else {
                    self.buffer[self.used..(self.used + to_write.len())].copy_from_slice(to_write);
                    self.used += to_write.len();
                    Ok(())
                }
            }
        }
        let mut w = WriteBuffer { buffer, used: 0 };
        core::fmt::write(&mut w, args)?;
        let b = &w.buffer[..w.used];
        // SAFETY: This is a concatenation of bytes of valid strs.
        let s = unsafe { core::str::from_utf8_unchecked(&b) };
        self.write_string(x, y, color, &s);
        Ok(())
    }
}
