use super::{
    canvas::Canvas,
    layer::{SharedWindow, WindowID},
    Color, Draw, ICoordinate, Point, Rectangle, Size, Vector2d, DESKTOP_BG_COLOR,
};

pub mod console;
pub mod text_window;

pub struct Widget<D: Draw> {
    layer: SharedWindow,
    draw: D,
}

impl<D: Draw> Widget<D> {
    pub fn new(layer: SharedWindow, draw: D) -> Self {
        Self { layer, draw }
    }

    pub fn set_draggable(&mut self, draggable: bool) {
        self.layer.lock().set_draggable(draggable);
    }

    pub fn window_id(&self) -> WindowID {
        self.layer.window_id()
    }

    pub fn move_relative(&mut self, v: Vector2d) -> (Point, Point) {
        let mut locked = self.layer.lock();
        locked.move_relative(v)
    }

    pub fn set_transparent_color(&mut self, transparent_color: Option<Color>) {
        self.layer.lock().set_transparent_color(transparent_color)
    }

    pub fn transparent_color(&mut self) -> Option<Color> {
        self.layer.lock().transparent_color()
    }

    pub fn draw_mut(&mut self) -> &mut D {
        &mut self.draw
    }

    pub fn buffer(&mut self) {
        let mut locked = self.layer.lock();
        locked.buffer(&self.draw);
    }
}

pub struct Desktop {
    size: Size,
}
impl Desktop {
    pub fn new(size: Size) -> Self {
        Self { size }
    }
}
impl Draw for Desktop {
    fn size(&self) -> Size {
        self.size
    }

    fn draw<C: Canvas>(&self, canvas: &mut C) {
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

pub struct Framed<D: Draw> {
    title: alloc::string::String,
    draw: D,
}
impl<D: Draw> Framed<D> {
    pub fn new(title: alloc::string::String, draw: D) -> Self {
        Self { title, draw }
    }
    pub fn draw_mut(&mut self) -> &mut D {
        &mut self.draw
    }
}
impl<D: Draw> Draw for Framed<D> {
    fn size(&self) -> Size {
        let inner = self.draw.size();
        Size::new(inner.x + 8, inner.y + 32)
    }

    fn draw<C: crate::graphics::canvas::Canvas>(&self, canvas: &mut C) {
        let size = self.size();
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
        let mut restricted = canvas.restricted(Rectangle::new(
            Point::new(4, 28),
            Size::new(size.x - 8, size.y - 32),
        ));
        self.draw.draw(&mut restricted);
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
