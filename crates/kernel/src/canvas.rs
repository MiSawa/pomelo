use snafu::{ResultExt as _, Snafu};

pub type Coordinate = i32;
const GLYPH_HEIGHT: Coordinate = 16;
const GLYPH_WIDTH: Coordinate = 8;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub struct Point {
    pub x: Coordinate,
    pub y: Coordinate,
}

impl Point {
    pub fn new(x: Coordinate, y: Coordinate) -> Self {
        Self { x, y }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
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

    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}

#[derive(Debug, Snafu)]
pub enum PaintError {
    #[snafu(display("Out of canvas"))]
    OutOfCanvas,
    #[snafu(display("Unable to format: {}", source))]
    FormatError { source: core::fmt::Error },
    #[snafu(display("Something went wrong: {}", message))]
    Whatever { message: &'static str },
}
pub type Result<T> = ::core::result::Result<T, PaintError>;

pub trait Canvas {
    fn width(&self) -> Coordinate;
    fn height(&self) -> Coordinate;
    fn draw_pixel(&mut self, p: Point, color: Color) -> Result<()>;

    fn draw_char(&mut self, p: Point, color: Color, c: char) -> Result<Coordinate> {
        use font8x8::legacy::{BASIC_LEGACY, NOTHING_TO_DISPLAY};
        let glyph = BASIC_LEGACY.get(c as usize).unwrap_or(&NOTHING_TO_DISPLAY);
        for (dy, row) in glyph
            .iter()
            .flat_map(|r| core::iter::repeat(*r).take(2))
            .enumerate()
        {
            for dx in 0..8 {
                if ((row >> dx) & 1) != 0 {
                    self.draw_pixel(Point::new(p.x + dx, p.y + dy as Coordinate), color)?;
                }
            }
        }
        Ok(GLYPH_WIDTH)
    }
    fn draw_string(&mut self, p: Point, color: Color, s: &str) -> Result<Coordinate> {
        let mut dx = 0;
        for c in s.chars() {
            dx += self.draw_char(Point::new(p.x + dx, p.y), color, c)?;
        }
        Ok(dx)
    }
    fn draw_fmt(
        &mut self,
        p: Point,
        color: Color,
        buffer: &mut [u8],
        args: core::fmt::Arguments,
    ) -> Result<Coordinate> {
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
        core::fmt::write(&mut w, args).context(FormatError {})?;
        let b = &w.buffer[..w.used];
        // SAFETY: This is a concatenation of bytes of valid strs.
        let s = unsafe { core::str::from_utf8_unchecked(b) };
        self.draw_string(p, color, s)
    }
}
