use alloc::string::String;

use crate::graphics::{
    buffer::VecBufferCanvas,
    canvas::{Canvas, GLYPH_HEIGHT, GLYPH_WIDTH},
    Color, ICoordinate, Point, Rectangle, Size, UCoordinate,
};

use super::Widget;

const TRANSPARENT_COLOR: Color = Color::new(1, 2, 3);

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

impl Widget for TextWindow {
    fn render(&self, canvas: &mut VecBufferCanvas) {
        canvas.resize(self.size);
        canvas.set_transparent_color(Some(TRANSPARENT_COLOR));
        canvas.fill_rectangle(self.background, Rectangle::new(Point::zero(), self.size));
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
