use alloc::string::String;

use crate::graphics::{
    canvas::{GLYPH_HEIGHT, GLYPH_WIDTH},
    Color, Draw, ICoordinate, Point, Rectangle, Size, UCoordinate,
};

pub struct TextWindow {
    buffer: String,
    foreground: Color,
    background: Color,
    size: Size,
    cursor_visible: bool,
}

impl TextWindow {
    pub fn new(foreground: Color, background: Color, len: usize) -> TextWindow {
        TextWindow {
            buffer: String::new(),
            foreground,
            background,
            size: Size::new(GLYPH_WIDTH * len as UCoordinate, GLYPH_HEIGHT),
            cursor_visible: false,
        }
    }

    pub fn push(&mut self, c: char) {
        self.buffer.push(c);
    }

    pub fn flip_cursor_visibility(&mut self) {
        self.cursor_visible ^= true;
    }
}

impl Draw for TextWindow {
    fn size(&self) -> crate::graphics::Size {
        self.size
    }

    fn draw<C: crate::graphics::canvas::Canvas>(&self, canvas: &mut C) {
        canvas.fill_rectangle(self.background, self.bounding_box());
        let i = canvas.draw_string(self.foreground, Point::zero(), &self.buffer);
        if self.cursor_visible {
            canvas.fill_rectangle(
                self.foreground,
                Rectangle::new(
                    Point::new(i as ICoordinate, 0),
                    Size::new(GLYPH_WIDTH, GLYPH_HEIGHT),
                ),
            );
        }
    }
}
