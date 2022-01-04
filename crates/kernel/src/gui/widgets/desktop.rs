use crate::{
    graphics::{
        buffer::VecBufferCanvas, canvas::Canvas, Color, ICoordinate, Point, Rectangle, Size,
    },
    gui::DESKTOP_BG_COLOR,
};

use super::Widget;

pub struct Desktop {
    size: Size,
}
impl Desktop {
    pub fn new(size: Size) -> Self {
        Self { size }
    }
}
impl Widget for Desktop {
    fn render(&self, canvas: &mut VecBufferCanvas) {
        canvas.resize(self.size);
        canvas.fill_rectangle(DESKTOP_BG_COLOR, canvas.bounding_box());
        canvas.fill_rectangle(
            Color::new(1, 8, 17),
            Rectangle::new(
                Point::new(0, self.size.y as ICoordinate - 50),
                Size::new(self.size.x, 50),
            ),
        );
        canvas.fill_rectangle(
            Color::new(80, 80, 80),
            Rectangle::new(
                Point::new(0, self.size.y as ICoordinate - 50),
                Size::new(self.size.x / 5, 50),
            ),
        );
        canvas.fill_rectangle(
            Color::new(160, 160, 160),
            Rectangle::new(
                Point::new(10, self.size.y as ICoordinate - 40),
                Size::new(30, 30),
            ),
        );
    }
}
