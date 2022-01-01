use delegate::delegate;
use pomelo_common::graphics::GraphicConfig;
use spinning_top::{MappedSpinlockGuard, Spinlock, SpinlockGuard};

use super::{
    buffer::{BufferCanvas, ByteBuffer},
    canvas::Canvas,
    Color, Point, Rectangle, Size, UCoordinate, Vector2d,
};

type ScreenRaw = BufferCanvas<FrameBuffer>;
lazy_static! {
    static ref SCREEN: Spinlock<Option<ScreenRaw>> = Spinlock::new(Option::None);
}

pub fn initialize(config: &GraphicConfig) {
    SCREEN.lock().get_or_insert_with(|| {
        ScreenRaw::new(
            FrameBuffer::new(config),
            config.pixel_format,
            Size::new(
                config.horisontal_resolution as UCoordinate,
                config.vertical_resolution as UCoordinate,
            ),
            config.stride,
        )
    });
}

pub struct FrameBuffer {
    inner: &'static mut [u8],
}
impl FrameBuffer {
    fn new(config: &GraphicConfig) -> Self {
        Self {
            inner: unsafe {
                core::slice::from_raw_parts_mut(config.frame_buffer_base, config.frame_buffer_size)
            },
        }
    }
}
impl ByteBuffer for FrameBuffer {
    fn as_slice(&self) -> &[u8] {
        self.inner
    }
    fn as_mut_slice(&mut self) -> &mut [u8] {
        self.inner
    }
}

pub fn screen() -> Screen {
    Screen()
}

pub struct ScreenLock<'a> {
    locked: MappedSpinlockGuard<'a, ScreenRaw>,
}
impl<'a> ScreenLock<'a> {
    fn new() -> Self {
        let locked = SCREEN.lock();
        let mapped = SpinlockGuard::map(locked, |screen| {
            screen.as_mut().expect("Screen should be initialized")
        });
        Self { locked: mapped }
    }

    fn unwrap(&self) -> &ScreenRaw {
        &self.locked
    }

    fn unwrap_mut(&mut self) -> &mut ScreenRaw {
        &mut self.locked
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
            fn draw_pixel_unchecked(&mut self, color: Color, p: Point);
            fn draw_pixel(&mut self, color: Color, p: Point);
            fn draw_buffer(&mut self, p: Vector2d, buffer: &BufferCanvas<impl ByteBuffer>);
            fn fill_rectangle(&mut self, color: Color, rectangle: &Rectangle);
            fn draw_char(&mut self, color: Color, p: Point, c: char) -> UCoordinate;
            fn draw_string(&mut self, color: Color, p: Point, s: &str) -> UCoordinate;
            fn draw_fmt(
                &mut self,
                color: Color,
                p: Point,
                args: core::fmt::Arguments,
            ) -> core::result::Result<UCoordinate, core::fmt::Error>;
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
            fn draw_pixel_unchecked(&mut self, color: Color, p: Point);
            fn draw_pixel(&mut self, color: Color, p: Point);
            fn draw_buffer(&mut self, p: Vector2d, buffer: &BufferCanvas<impl ByteBuffer>);
            fn fill_rectangle(&mut self, color: Color, rectangle: &Rectangle);
            fn draw_char(&mut self, color: Color, p: Point, c: char) -> UCoordinate;
            fn draw_string(&mut self, color: Color, p: Point, s: &str) -> UCoordinate;
            fn draw_fmt(
                &mut self,
                color: Color,
                p: Point,
                args: core::fmt::Arguments,
            ) -> core::result::Result<UCoordinate, core::fmt::Error>;
        }
    }
}
