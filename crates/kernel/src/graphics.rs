use self::canvas::Canvas;

pub mod buffer;
pub(crate) mod canvas;
pub(crate) mod screen;

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
    #[must_use]
    pub fn clamped(&self, r: Rectangle) -> Point {
        let mut p = *self;
        p.x = p.x.clamp(r.min_x(), r.max_x());
        p.y = p.y.clamp(r.min_x(), r.max_x());
        p
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub struct Vector2d {
    pub x: ICoordinate,
    pub y: ICoordinate,
}

impl Vector2d {
    pub const fn zero() -> Self {
        Self::new(0, 0)
    }
    pub const fn new(x: ICoordinate, y: ICoordinate) -> Self {
        Self { x, y }
    }
}

impl From<Point> for Vector2d {
    fn from(p: Point) -> Self {
        Vector2d::new(p.x, p.y)
    }
}

impl From<Vector2d> for Point {
    fn from(v: Vector2d) -> Self {
        Point::new(v.x, v.y)
    }
}

impl core::ops::Add<Vector2d> for Point {
    type Output = Point;

    fn add(self, rhs: Vector2d) -> Self::Output {
        Self::Output::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl core::ops::Sub<Vector2d> for Point {
    type Output = Point;

    fn sub(self, rhs: Vector2d) -> Self::Output {
        Self::Output::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl core::ops::Sub<Vector2d> for Vector2d {
    type Output = Vector2d;

    fn sub(self, rhs: Vector2d) -> Self::Output {
        Self::Output::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl core::ops::Sub<Point> for Point {
    type Output = Vector2d;

    fn sub(self, rhs: Point) -> Self::Output {
        Self::Output::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl core::ops::AddAssign<Vector2d> for Point {
    fn add_assign(&mut self, rhs: Vector2d) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl core::ops::Add<Vector2d> for Vector2d {
    type Output = Vector2d;

    fn add(self, rhs: Vector2d) -> Self::Output {
        Self::Output::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl core::ops::AddAssign<Vector2d> for Vector2d {
    fn add_assign(&mut self, rhs: Vector2d) {
        self.x += rhs.x;
        self.y += rhs.y;
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

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
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
    pub fn from_corner(top_left: Point, bottom_right: Point) -> Self {
        Self {
            top_left,
            size: Size::new(
                (bottom_right.x - top_left.x).try_into().unwrap(),
                (bottom_right.y - top_left.y).try_into().unwrap(),
            ),
        }
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
    pub fn points(&self) -> impl Iterator<Item = Point> + '_ {
        self.ys()
            .flat_map(|y| self.xs().map(move |x| Point { x, y }))
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
    #[must_use]
    pub fn union(&self, other: &Self) -> Self {
        let min_x = self.min_x().min(other.min_x());
        let max_x = self.max_x().max(other.max_x());
        let min_y = self.min_y().min(other.min_y());
        let max_y = self.max_y().max(other.max_y());
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

impl core::ops::Add<Vector2d> for Rectangle {
    type Output = Rectangle;

    fn add(self, rhs: Vector2d) -> Self::Output {
        Self::Output::new(self.top_left + rhs, self.size)
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

    pub const fn gray_scale(v: u8) -> Self {
        Self::new(v, v, v)
    }

    pub const fn from_code(v: u32) -> Self {
        Self::new((v >> 16) as u8, (v >> 8) as u8, v as u8)
    }
}

pub trait Draw {
    fn size(&self) -> Size;
    fn bounding_box(&self) -> Rectangle {
        Rectangle::new(Point::zero(), self.size())
    }
    fn draw<C: Canvas>(&self, canvas: &mut C);
}
