use delegate::delegate;
use pomelo_common::GraphicConfig;
use spin::Mutex;

use crate::graphics::{
    canvas::{Canvas, Result},
    Color, ICoordinate, Point, Rectangle, Size, UCoordinate,
};

lazy_static! {
    static ref SCREEN: Mutex<Option<ScreenRaw>> = Mutex::new(Option::None);
}

pub fn initialize(graphic_config: &GraphicConfig) {
    SCREEN.lock().replace(ScreenRaw::from(graphic_config));
}

pub fn screen() -> Screen {
    Screen()
}

struct ScreenRaw {
    buffer: &'static mut [u8],
    r_offset: u8,
    g_offset: u8,
    b_offset: u8,
    horisontal_resolution: usize,
    vertical_resolution: usize,
    stride: usize,
}

impl ScreenRaw {
    pub fn from(config: &GraphicConfig) -> Self {
        Self {
            buffer: unsafe {
                core::slice::from_raw_parts_mut(config.frame_buffer_base, config.frame_buffer_size)
            },
            r_offset: config.pixel_format.r_offset(),
            g_offset: config.pixel_format.g_offset(),
            b_offset: config.pixel_format.b_offset(),
            horisontal_resolution: config.horisontal_resolution,
            vertical_resolution: config.vertical_resolution,
            stride: config.stride,
        }
    }

    fn offset_of_pixel(&self, p: Point) -> usize {
        4 * (self.stride * (p.y as usize) + (p.x as usize))
    }
}

impl Canvas for ScreenRaw {
    fn size(&self) -> Size {
        Size::new(
            self.horisontal_resolution as UCoordinate,
            self.vertical_resolution as UCoordinate,
        )
    }

    fn draw_pixel(&mut self, color: Color, p: Point) -> Result<()> {
        let size = self.size();
        if p.x < 0 || p.x >= (size.x as ICoordinate) || p.y < 0 || p.y >= (size.y as ICoordinate) {
            return Ok(());
        }
        let offset = self.offset_of_pixel(p);
        self.buffer[offset + self.r_offset as usize] = color.r;
        self.buffer[offset + self.g_offset as usize] = color.g;
        self.buffer[offset + self.b_offset as usize] = color.b;
        Ok(())
    }

    fn fill_rectangle(&mut self, color: Color, rectangle: &Rectangle) -> Result<()> {
        let rectangle = rectangle.intersection(&self.bounding_box());
        let mut pattern = [0; 4];
        pattern[self.r_offset as usize] = color.r;
        pattern[self.g_offset as usize] = color.g;
        pattern[self.b_offset as usize] = color.b;

        let mut s = self.offset_of_pixel(rectangle.top_left());
        let mut t = self.offset_of_pixel(rectangle.top_right());
        for _ in 0..rectangle.height() {
            for i in (s..t).step_by(4) {
                self.buffer[i..(i + 4)].copy_from_slice(&pattern);
            }
            s += self.stride * 4;
            t += self.stride * 4;
        }
        Ok(())
    }
}

pub struct ScreenLock<'a> {
    locked: spin::mutex::MutexGuard<'a, Option<ScreenRaw>>,
}
impl<'a> ScreenLock<'a> {
    fn new() -> Self {
        let locked = SCREEN.lock();
        Self { locked }
    }

    fn unwrap(&self) -> &ScreenRaw {
        self.locked.as_ref().unwrap()
    }

    fn unwrap_mut(&mut self) -> &mut ScreenRaw {
        self.locked.as_mut().unwrap()
    }
}

pub struct Screen();
impl Screen {
    pub fn lock(&self) -> impl Canvas {
        ScreenLock::new()
    }
}

impl<'a> Canvas for ScreenLock<'a> {
    delegate! {
        to self.unwrap() {
            fn size(&self) -> Size;
            fn width(&self) -> UCoordinate;
            fn height(&self) -> UCoordinate;
            fn bounding_box(&self) -> Rectangle;
        }
        to self.unwrap_mut() {
            fn draw_pixel(&mut self, color: Color, p: Point) -> Result<()>;
            fn fill_rectangle(&mut self, color: Color, rectangle: &Rectangle) -> Result<()> ;
            fn draw_char(&mut self, color: Color, p: Point, c: char) -> Result<ICoordinate>;
            fn draw_string(&mut self, color: Color, p: Point, s: &str) -> Result<ICoordinate>;
            fn draw_fmt(
                &mut self,
                color: Color,
                p: Point,
                buffer: &mut [u8],
                args: core::fmt::Arguments,
            ) -> Result<ICoordinate>;
        }
    }
}

impl Canvas for Screen {
    delegate! {
        to self.lock() {
            fn size(&self) -> Size;
            fn width(&self) -> UCoordinate;
            fn height(&self) -> UCoordinate;
            fn bounding_box(&self) -> Rectangle;
            fn draw_pixel(&mut self, color: Color, p: Point) -> Result<()>;
            fn fill_rectangle(&mut self, color: Color, rectangle: &Rectangle) -> Result<()> ;
            fn draw_char(&mut self, color: Color, p: Point, c: char) -> Result<ICoordinate>;
            fn draw_string(&mut self, color: Color, p: Point, s: &str) -> Result<ICoordinate>;
            fn draw_fmt(
                &mut self,
                color: Color,
                p: Point,
                buffer: &mut [u8],
                args: core::fmt::Arguments,
            ) -> Result<ICoordinate>;
        }
    }
}