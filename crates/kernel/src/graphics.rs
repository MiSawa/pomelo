pub mod canvas;
pub mod console;
pub mod mouse;
pub mod screen;

pub type ICoordinate = i32;
pub type UCoordinate = u32;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub struct Point {
    pub x: ICoordinate,
    pub y: ICoordinate,
}

impl Point {
    pub const fn zero() -> Self {
        Self::new(0, 0)
    }
    pub const fn new(x: ICoordinate, y: ICoordinate) -> Self {
        Self { x, y }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub struct Size {
    pub x: UCoordinate,
    pub y: UCoordinate,
}

impl Size {
    pub const fn zero() -> Self {
        Self::new(0, 0)
    }
    pub const fn new(x: UCoordinate, y: UCoordinate) -> Self {
        Self { x, y }
    }
}

pub struct Rectangle {
    top_left: Point,
    size: Size,
}

impl Rectangle {
    pub const fn empty() -> Self {
        Self {
            top_left: Point::zero(),
            size: Size::zero(),
        }
    }
    pub const fn new(top_left: Point, size: Size) -> Self {
        Self { top_left, size }
    }
    pub const fn contains(&self, p: &Point) -> bool {
        self.min_x() <= p.x && p.x < self.max_x() && self.min_y() <= p.y && p.y < self.max_y()
    }
    pub const fn min_x(&self) -> ICoordinate {
        self.top_left.x
    }
    pub const fn min_y(&self) -> ICoordinate {
        self.top_left.y
    }
    pub const fn max_x(&self) -> ICoordinate {
        self.top_left.x + (self.size.x as ICoordinate)
    }
    pub const fn max_y(&self) -> ICoordinate {
        self.top_left.y + (self.size.y as ICoordinate)
    }
    pub const fn top_left(&self) -> Point {
        self.top_left
    }
    pub const fn top_right(&self) -> Point {
        Point::new(self.max_x(), self.min_y())
    }
    pub const fn bottom_left(&self) -> Point {
        Point::new(self.min_x(), self.max_y())
    }
    pub const fn bottom_right(&self) -> Point {
        Point::new(self.max_x(), self.max_y())
    }
    pub const fn width(&self) -> UCoordinate {
        self.size.x
    }
    pub const fn height(&self) -> UCoordinate {
        self.size.y
    }
    pub fn xs(&self) -> impl Iterator<Item = ICoordinate> {
        self.min_x()..self.max_x()
    }
    pub fn ys(&self) -> impl Iterator<Item = ICoordinate> {
        self.min_y()..self.max_y()
    }
    #[must_use]
    pub fn intersection(&self, other: &Self) -> Self {
        let min_x = self.min_x().max(other.min_x());
        let max_x = self.max_x().min(other.max_x());
        let min_y = self.min_y().max(other.min_y());
        let max_y = self.max_y().min(other.max_y());
        if min_x <= max_x && min_y <= max_y {
            Self::new(
                Point::new(min_x, min_y),
                Size::new(
                    (max_x - min_x) as UCoordinate,
                    (max_y - min_y) as UCoordinate,
                ),
            )
        } else {
            Self::empty()
        }
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
    pub const BLACK: Color = Color::new(0, 0, 0);
    pub const WHITE: Color = Color::new(255, 255, 255);
    pub const RED: Color = Color::new(255, 0, 0);
    pub const GREEN: Color = Color::new(0, 255, 0);
    pub const BLUE: Color = Color::new(0, 0, 255);

    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}
