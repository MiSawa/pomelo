use super::{
    canvas::Canvas, layer::SharedWindow, Color, Draw, ICoordinate, Point, Rectangle, Size,
    Vector2d, DESKTOP_BG_COLOR,
};

pub mod console;
pub mod text;

pub struct Widget<D: Draw> {
    layer: SharedWindow,
    draw: D,
}

impl<D: Draw> Widget<D> {
    pub fn new(layer: SharedWindow, draw: D) -> Self {
        Self { layer, draw }
    }

    pub fn move_relative(&mut self, v: Vector2d) {
        let mut locked = self.layer.lock();
        locked.move_relative(v);
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
        canvas.fill_rectangle(DESKTOP_BG_COLOR, &canvas.bounding_box());
        canvas.fill_rectangle(
            Color::new(1, 8, 17),
            &Rectangle::new(
                Point::new(0, self.size.y as ICoordinate - 50),
                Size::new(self.size.x, 50),
            ),
        );
        canvas.fill_rectangle(
            Color::new(80, 80, 80),
            &Rectangle::new(
                Point::new(0, self.size.y as ICoordinate - 50),
                Size::new(self.size.x / 5, 50),
            ),
        );
        canvas.fill_rectangle(
            Color::new(160, 160, 160),
            &Rectangle::new(
                Point::new(10, self.size.y as ICoordinate - 40),
                Size::new(30, 30),
            ),
        );
    }
}
