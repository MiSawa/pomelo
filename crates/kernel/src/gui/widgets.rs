use crate::graphics::{
    buffer::VecBufferCanvas, canvas::Canvas, Color, ICoordinate, Point, Rectangle, Size,
    Vector2d,
};

pub mod console;
pub mod desktop;
pub mod text_window;

pub trait Widget {
    fn render(&self, canvas: &mut VecBufferCanvas);
}

pub struct Framed<W: Widget> {
    title: alloc::string::String,
    widget: W,
}
impl<W: Widget> Framed<W> {
    pub fn new(title: alloc::string::String, widget: W) -> Self {
        Self { title, widget }
    }
    pub fn widget_mut(&mut self) -> &mut W {
        &mut self.widget
    }
}
impl<W: Widget> Widget for Framed<W> {
    fn render(&self, canvas: &mut VecBufferCanvas) {
        let mut buffer = VecBufferCanvas::empty(canvas.pixel_format());
        self.widget.render(&mut buffer);
        let mut size = buffer.size();
        size.x += 8;
        size.y += 32;
        canvas.resize(size);
        canvas.fill_rectangle(
            Color::gray_scale(0xC6),
            Rectangle::new(Point::new(0, 0), Size::new(size.x, 1)),
        );
        canvas.fill_rectangle(
            Color::gray_scale(0xFF),
            Rectangle::new(Point::new(1, 1), Size::new(size.x - 2, 1)),
        );
        canvas.fill_rectangle(
            Color::gray_scale(0xC6),
            Rectangle::new(Point::new(0, 0), Size::new(1, size.y)),
        );
        canvas.fill_rectangle(
            Color::gray_scale(0xFF),
            Rectangle::new(Point::new(1, 1), Size::new(1, size.y - 2)),
        );
        canvas.fill_rectangle(
            Color::gray_scale(0x84),
            Rectangle::new(
                Point::new(size.x as ICoordinate - 2, 1),
                Size::new(1, size.y - 2),
            ),
        );
        canvas.fill_rectangle(
            Color::gray_scale(0x00),
            Rectangle::new(
                Point::new(size.x as ICoordinate - 1, 0),
                Size::new(1, size.y),
            ),
        );
        canvas.fill_rectangle(
            Color::gray_scale(0xC6),
            Rectangle::new(Point::new(2, 2), Size::new(size.x - 4, size.y - 4)),
        );
        canvas.fill_rectangle(
            Color::new(0, 0, 0x84),
            Rectangle::new(Point::new(3, 3), Size::new(size.x - 6, 18)),
        );
        canvas.fill_rectangle(
            Color::gray_scale(0x84),
            Rectangle::new(
                Point::new(1, size.y as ICoordinate - 2),
                Size::new(size.x - 2, 1),
            ),
        );
        canvas.fill_rectangle(
            Color::gray_scale(0x00),
            Rectangle::new(
                Point::new(0, size.y as ICoordinate - 1),
                Size::new(size.x, 1),
            ),
        );
        canvas.draw_string(Color::WHITE, Point::new(24, 4), &self.title);
        for (y, row) in CLOSE_BUTTON.iter().enumerate() {
            for (x, c) in row.iter().enumerate() {
                let color = match c {
                    b'@' => Color::gray_scale(0x00),
                    b'$' => Color::gray_scale(0x84),
                    b':' => Color::gray_scale(0xC6),
                    _ => Color::gray_scale(0xFF),
                };
                canvas.draw_pixel(
                    color,
                    Point::new(
                        size.x as ICoordinate - 5 - CLOSE_BUTTON_WIDTH as ICoordinate
                            + x as ICoordinate,
                        5 + y as ICoordinate,
                    ),
                );
            }
        }
        canvas.draw_buffer(Vector2d::new(4, 28), &buffer)
    }
}

const CLOSE_BUTTON_WIDTH: usize = 16;
const CLOSE_BUTTON_HEIGHT: usize = 14;
const CLOSE_BUTTON: [[u8; CLOSE_BUTTON_WIDTH]; CLOSE_BUTTON_HEIGHT] = [
    *b"...............@",
    *b".:::::::::::::$@",
    *b".:::::::::::::$@",
    *b".:::@@::::@@::$@",
    *b".::::@@::@@:::$@",
    *b".:::::@@@@::::$@",
    *b".::::::@@:::::$@",
    *b".:::::@@@@::::$@",
    *b".::::@@::@@:::$@",
    *b".:::@@::::@@::$@",
    *b".:::::::::::::$@",
    *b".:::::::::::::$@",
    *b".$$$$$$$$$$$$$$@",
    *b"@@@@@@@@@@@@@@@@",
];
