use pomelo_common::GraphicConfig;
use spin::Mutex;

use crate::canvas::{Canvas, Color, Coordinate, PaintError, Point, Result};

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
}
impl Canvas for ScreenRaw {
    fn width(&self) -> Coordinate {
        self.horisontal_resolution as Coordinate
    }

    fn height(&self) -> Coordinate {
        self.vertical_resolution as Coordinate
    }

    fn draw_pixel(&mut self, p: Point, color: Color) -> Result<()> {
        if p.x < 0 || p.x >= self.width() || p.y < 0 || p.y >= self.height() {
            return Err(PaintError::OutOfCanvas);
        }
        let pos = 4 * (self.stride * (p.y as usize) + (p.x as usize));
        self.buffer[pos + self.r_offset as usize] = color.r;
        self.buffer[pos + self.g_offset as usize] = color.g;
        self.buffer[pos + self.b_offset as usize] = color.b;
        Ok(())
    }
}

pub struct ScreenLock<'a> {
    lock: spin::mutex::MutexGuard<'a, Option<ScreenRaw>>,
}
impl<'a> ScreenLock<'a> {
    fn new() -> Self {
        let lock = SCREEN.lock();
        Self { lock }
    }

    fn unwrap(&self) -> &ScreenRaw {
        self.lock.as_ref().unwrap()
    }

    fn unwrap_mut(&mut self) -> &mut ScreenRaw {
        self.lock.as_mut().unwrap()
    }
}

pub struct Screen();
impl Screen {
    pub fn lock(&self) -> ScreenLock {
        ScreenLock::new()
    }
}

impl<'a> Canvas for ScreenLock<'a> {
    fn width(&self) -> Coordinate {
        self.unwrap().width()
    }

    fn height(&self) -> Coordinate {
        self.unwrap().height()
    }

    fn draw_pixel(&mut self, p: Point, color: Color) -> Result<()> {
        self.unwrap_mut().draw_pixel(p, color)
    }
}

impl Canvas for Screen {
    fn width(&self) -> Coordinate {
        self.lock().width()
    }

    fn height(&self) -> Coordinate {
        self.lock().height()
    }

    fn draw_pixel(&mut self, p: Point, color: Color) -> Result<()> {
        self.lock().draw_pixel(p, color)
    }

    fn draw_char(&mut self, p: Point, color: Color, c: char) -> Result<Coordinate> {
        self.lock().draw_char(p, color, c)
    }

    fn draw_string(&mut self, p: Point, color: Color, s: &str) -> Result<Coordinate> {
        self.lock().draw_string(p, color, s)
    }

    fn draw_fmt(
        &mut self,
        p: Point,
        color: Color,
        buffer: &mut [u8],
        args: core::fmt::Arguments,
    ) -> Result<Coordinate> {
        self.lock().draw_fmt(p, color, buffer, args)
    }
}
