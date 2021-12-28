use snafu::{ResultExt as _, Snafu};

use crate::graphics::{Color, ICoordinate, Point, Rectangle, Size, UCoordinate};

pub const GLYPH_HEIGHT: ICoordinate = 16;
pub const GLYPH_WIDTH: ICoordinate = 8;

#[derive(Debug, Snafu)]
pub enum PaintError {
    #[snafu(display("Unable to format: {}", source))]
    FormatError { source: core::fmt::Error },
    #[snafu(display("Something went wrong: {}", message))]
    Whatever { message: &'static str },
}
pub type Result<T> = ::core::result::Result<T, PaintError>;

pub trait Canvas {
    fn size(&self) -> Size;
    fn width(&self) -> UCoordinate {
        self.size().x
    }
    fn height(&self) -> UCoordinate {
        self.size().y
    }
    fn bounding_box(&self) -> Rectangle {
        Rectangle::new(Point::zero(), self.size())
    }

    fn draw_pixel(&mut self, color: Color, p: Point) -> Result<()>;
    fn fill_rectangle(&mut self, color: Color, rectangle: &Rectangle) -> Result<()> {
        for y in rectangle.ys() {
            for x in rectangle.xs() {
                self.draw_pixel(color, Point::new(x, y))?
            }
        }
        Ok(())
    }
    fn draw_char(&mut self, color: Color, p: Point, c: char) -> Result<ICoordinate> {
        use font8x8::legacy::{BASIC_LEGACY, NOTHING_TO_DISPLAY};
        let glyph = BASIC_LEGACY.get(c as usize).unwrap_or(&NOTHING_TO_DISPLAY);
        for (dy, row) in glyph
            .iter()
            .flat_map(|r| core::iter::repeat(*r).take(2))
            .enumerate()
        {
            for dx in 0..8 {
                if ((row >> dx) & 1) != 0 {
                    self.draw_pixel(color, Point::new(p.x + dx, p.y + dy as ICoordinate))?;
                }
            }
        }
        Ok(GLYPH_WIDTH)
    }
    fn draw_string(&mut self, color: Color, p: Point, s: &str) -> Result<ICoordinate> {
        let mut dx = 0;
        for c in s.chars() {
            dx += self.draw_char(color, Point::new(p.x + dx, p.y), c)?;
        }
        Ok(dx)
    }
    fn draw_fmt(
        &mut self,
        color: Color,
        p: Point,
        buffer: &mut [u8],
        args: core::fmt::Arguments,
    ) -> Result<ICoordinate> {
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
        self.draw_string(color, p, s)
    }
}
