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

    pub fn write_char(&mut self, x: usize, y: usize, c: u8, color: &Color) {
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
}
