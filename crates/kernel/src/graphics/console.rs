const ROWS: usize = 25;
const COLUMNS: usize = 80;

use arrayvec::{ArrayString, ArrayVec};

use crate::graphics::{
    canvas::{Canvas, GLYPH_HEIGHT, GLYPH_WIDTH},
    Color, ICoordinate, Point,
};

pub struct Console<C: Canvas> {
    canvas: C,
    buffer: ArrayVec<ArrayString<{ COLUMNS * 4 }>, ROWS>,
    foreground: Color,
    background: Color,
    cursor_point: Point,
}

impl<C: Canvas> Console<C> {
    pub fn new(canvas: C, foreground: Color, background: Color) -> Self {
        let mut buffer = ArrayVec::new();
        buffer.push(ArrayString::new());
        Self {
            buffer,
            canvas,
            foreground,
            background,
            cursor_point: Point::zero(),
        }
    }

    fn rc_to_point(row: usize, column: usize) -> Point {
        Point::new(
            column as ICoordinate * GLYPH_WIDTH,
            row as ICoordinate * GLYPH_HEIGHT,
        )
    }

    fn new_line(&mut self) {
        // TODO: Use buffer as a ring buffer.
        if self.buffer.is_full() {
            self.canvas
                .fill_rectangle(self.background, &self.canvas.bounding_box())
                .ok();
            self.buffer.remove(0);
            for (i, row) in self.buffer.iter().enumerate() {
                self.canvas
                    .draw_string(self.foreground, Self::rc_to_point(i, 0), row)
                    .ok();
            }
        }
        self.buffer.push(ArrayString::new());
        self.cursor_point = Self::rc_to_point(self.buffer.len() - 1, 0);
    }

    pub fn write_string(&mut self, s: &str) {
        for c in s.chars() {
            if c == '\n' {
                self.new_line();
            } else {
                self.cursor_point.x += self
                    .canvas
                    .draw_char(self.foreground, self.cursor_point, c)
                    .unwrap_or(0);
                self.buffer.last_mut().unwrap().try_push(c).ok();
            }
        }
    }
}

impl<C: Canvas> core::fmt::Write for Console<C> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.write_string(s);
        Ok(())
    }
}
