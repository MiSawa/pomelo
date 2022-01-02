extern crate alloc;

use alloc::{sync::Arc, vec, vec::Vec};
use pomelo_common::graphics::{GraphicConfig, PixelFormat};
use spinning_top::Spinlock;

use super::{
    buffer::BufferCanvas, canvas::Canvas, widgets::Widget, Color, Draw, ICoordinate, Size,
    UCoordinate, Vector2d,
};

pub fn create_layer_manager(graphic_config: &GraphicConfig) -> LayerManager {
    LayerManager::new(
        graphic_config.pixel_format,
        Size::new(
            graphic_config.horisontal_resolution as UCoordinate,
            graphic_config.vertical_resolution as UCoordinate,
        ),
    )
}

type WindowBuffer = BufferCanvas<Vec<u8>>;
pub(crate) type SharedWindow = Arc<Spinlock<Window>>;

pub struct Window {
    position: Vector2d,
    buffer: WindowBuffer,
    container_size: Size,
}

impl Window {
    fn new(pixel_format: PixelFormat, size: Size, container_size: Size) -> Self {
        Self {
            position: Vector2d::zero(),
            buffer: WindowBuffer::vec_backed(pixel_format, size),
            container_size,
        }
    }

    pub fn move_relative(&mut self, v: Vector2d) {
        self.position += v;
        self.position.x = self
            .position
            .x
            .clamp(0, self.container_size.x as ICoordinate);
        self.position.y = self
            .position
            .y
            .clamp(0, self.container_size.y as ICoordinate);
    }

    pub fn set_transparent_color(&mut self, transparent_color: Option<Color>) {
        self.buffer.set_transparent_color(transparent_color)
    }

    pub fn transparent_color(&self) -> Option<Color> {
        self.buffer.transparent_color()
    }

    pub fn buffer<D: Draw>(&mut self, draw: &D) {
        draw.draw(&mut self.buffer);
    }
}

pub enum MaybeRegistered<D: Draw> {
    Unregistered(D),
    Registered(Widget<D>),
    // Ugggh I want replace_with
    Registering,
}

impl<D: Draw> MaybeRegistered<D> {
    pub fn register_once(&mut self, layer_manager: &mut LayerManager) -> &mut Widget<D> {
        let took = core::mem::replace(self, Self::Registering);
        *self = match took {
            MaybeRegistered::Unregistered(d) => Self::Registered(layer_manager.add(d)),
            other => other,
        };
        if let MaybeRegistered::Registered(w) = self {
            w
        } else {
            panic!("Whaaat, maybe it failed to register itself?")
        }
    }
    pub fn unwrap_mut(&mut self) -> &mut D {
        match self {
            MaybeRegistered::Unregistered(d) => d,
            MaybeRegistered::Registered(w) => w.draw_mut(),
            _ => panic!("Whaaat, maybe it failed to register itself?"),
        }
    }
    pub fn buffer(&mut self) {
        if let MaybeRegistered::Registered(w) = self {
            w.buffer();
        }
    }
    pub fn get_widget(&mut self) -> Option<&mut Widget<D>> {
        if let MaybeRegistered::Registered(w) = self {
            Some(w)
        } else {
            None
        }
    }
}

pub struct LayerManager {
    pixel_format: PixelFormat,
    layers: Vec<SharedWindow>,
    size: Size,
}

impl LayerManager {
    fn new(pixel_format: PixelFormat, size: Size) -> Self {
        Self {
            pixel_format,
            layers: vec![],
            size,
        }
    }

    pub fn add<D: Draw>(&mut self, draw: D) -> Widget<D> {
        let mut window = Window::new(self.pixel_format, draw.size(), self.size);
        draw.draw(&mut window.buffer);
        let shared = Arc::new(Spinlock::new(window));
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
