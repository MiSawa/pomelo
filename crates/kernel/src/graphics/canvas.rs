use crate::graphics::{Color, ICoordinate, Point, Rectangle, Size, UCoordinate};

use super::{
    buffer::{BufferCanvas, ByteBuffer},
    Vector2d,
};

pub const GLYPH_HEIGHT: UCoordinate = 16;
pub const GLYPH_WIDTH: UCoordinate = 8;

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
    fn restricted(&mut self, area: Rectangle) -> RestrictedCanvas<'_, Self>
    where
        Self: Sized,
    {
        RestrictedCanvas { outer: self, area }
    }

    fn draw_pixel_unchecked(&mut self, color: Color, p: Point);

    fn draw_pixel(&mut self, color: Color, p: Point) {
        if self.bounding_box().contains(&p) {
            self.draw_pixel_unchecked(color, p);
        }
    }

    fn draw_buffer(&mut self, v: Vector2d, buffer: &BufferCanvas<impl ByteBuffer>) {
        let rectangle = (buffer.bounding_box() + v).intersection(&self.bounding_box());
        for p in rectangle.points() {
            if let Some(c) = buffer.get_color(p - v) {
                self.draw_pixel_unchecked(c, p);
            }
        }
    }

    fn fill_rectangle(&mut self, color: Color, rectangle: &Rectangle) {
        let actual = self.bounding_box().intersection(rectangle);
        for p in actual.points() {
            self.draw_pixel_unchecked(color, p)
        }
    }
    fn draw_char(&mut self, color: Color, p: Point, c: char) -> UCoordinate {
        use font8x8::legacy::{BASIC_LEGACY, NOTHING_TO_DISPLAY};
        let glyph = BASIC_LEGACY.get(c as usize).unwrap_or(&NOTHING_TO_DISPLAY);
        for (dy, row) in glyph
            .iter()
            .flat_map(|r| core::iter::repeat(*r).take(2))
            .enumerate()
        {
            for dx in 0..8 {
                if ((row >> dx) & 1) != 0 {
                    self.draw_pixel(color, Point::new(p.x + dx, p.y + dy as ICoordinate));
                }
            }
        }
        GLYPH_WIDTH
    }
    fn draw_string(&mut self, color: Color, p: Point, s: &str) -> UCoordinate {
        let mut dx = 0;
        for c in s.chars() {
            dx += self.draw_char(color, Point::new(p.x + (dx as ICoordinate), p.y), c);
        }
        dx
    }

    fn draw_fmt(
        &mut self,
        color: Color,
        p: Point,
        args: core::fmt::Arguments,
    ) -> core::result::Result<UCoordinate, core::fmt::Error> {
        struct WriteBuffer<'a, C: Canvas + ?Sized> {
            canvas: &'a mut C,
            color: Color,
            p: Point,
            dx: UCoordinate,
        }
        impl<'a, C: Canvas + ?Sized> core::fmt::Write for WriteBuffer<'a, C> {
            fn write_str(&mut self, s: &str) -> core::fmt::Result {
                self.dx += self.canvas.draw_string(
                    self.color,
                    Point::new(self.p.x + self.dx as ICoordinate, self.p.y),
                    s,
                );
                Ok(())
            }
        }
        let mut w = WriteBuffer {
            canvas: self,
            color,
            p,
            dx: 0,
        };
        core::fmt::write(&mut w, args)?;
        Ok(w.dx)
    }
}

pub struct RestrictedCanvas<'a, C: Canvas> {
    outer: &'a mut C,
    area: Rectangle,
}

impl<'a, C: Canvas> Canvas for RestrictedCanvas<'a, C> {
    fn size(&self) -> Size {
        self.area.size
    }

    fn draw_pixel_unchecked(&mut self, color: Color, p: Point) {
        let q = p + self.area.top_left.into();
        crate::println!("({}, {})", q.x, q.y);
        self.outer
            .draw_pixel_unchecked(color, p + self.area.top_left.into());
    }

    fn fill_rectangle(&mut self, color: Color, rectangle: &Rectangle) {
        let actual = self.bounding_box().intersection(rectangle);
        self.outer
            .fill_rectangle(color, &(actual + self.area.top_left.into()))
    }
}
