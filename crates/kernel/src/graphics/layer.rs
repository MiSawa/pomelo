extern crate alloc;

use alloc::{rc::Rc, vec, vec::Vec};
use pomelo_common::graphics::{GraphicConfig, PixelFormat};
use spinning_top::Spinlock;

use super::{Draw, Size, UCoordinate, Vector2d, buffer::BufferCanvas, canvas::Canvas, widgets::Widget};

type WindowBuffer = BufferCanvas<Vec<u8>>;
pub(crate) type SharedWindow = Rc<Spinlock<Window>>;

pub fn initialize(graphic_config: &GraphicConfig) -> LayerManager {
    LayerManager::new(
        graphic_config.pixel_format,
        Size::new(
            graphic_config.horisontal_resolution as UCoordinate,
            graphic_config.vertical_resolution as UCoordinate,
        ),
    )
}

pub struct Window {
    position: Vector2d,
    buffer: WindowBuffer,
}

impl Window {
    fn new(pixel_format: PixelFormat, size: Size) -> Self {
        Self {
            position: Vector2d::zero(),
            buffer: WindowBuffer::vec_backed(pixel_format, size),
        }
    }
}

pub struct LayerManager {
    pixel_format: PixelFormat,
    layers: Vec<SharedWindow>,
    size: Size,
}

impl LayerManager {
    pub fn new(pixel_format: PixelFormat, size: Size) -> Self {
        Self {
            pixel_format,
            layers: vec![],
            size,
        }
    }

    pub fn add<D: Draw>(&mut self, draw: D) -> Widget<D> {
        let window = Window::new(self.pixel_format, draw.size());
        let shared = Rc::new(Spinlock::new(window));
        self.layers.push(shared.clone());
        Widget::new(shared, draw)
    }
}

impl Draw for LayerManager {
    fn size(&self) -> Size {
        self.size
    }

    fn draw<C: Canvas>(&self, canvas: &mut C) {
        for layer in &self.layers {
            let layer = layer.lock();
            canvas.draw_buffer(layer.position, &layer.buffer);
        }
    }
}
