extern crate alloc;

use core::sync::atomic::{AtomicUsize, Ordering};

use alloc::{sync::Arc, vec, vec::Vec};
use pomelo_common::graphics::{GraphicConfig, PixelFormat};
use spinning_top::Spinlock;

use super::{
    buffer::BufferCanvas, canvas::Canvas, widgets::Widget, Color, Draw, ICoordinate, Rectangle,
    Size, UCoordinate, Vector2d,
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

#[derive(Clone)]
pub struct SharedWindow {
    id: WindowID,
    inner: Arc<Spinlock<Window>>,
}
impl SharedWindow {
    fn new(window: Window) -> Self {
        Self {
            id: window.id,
            inner: Arc::new(Spinlock::new(window)),
        }
    }
    pub fn lock(&self) -> spinning_top::SpinlockGuard<'_, Window> {
        self.inner.lock()
    }

    pub fn window_id(&self) -> WindowID {
        self.id
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct WindowID(usize);
impl WindowID {
    pub fn generate() -> Self {
        static WINDOW_ID_GENERATOR: AtomicUsize = AtomicUsize::new(0);
        let id = WINDOW_ID_GENERATOR.fetch_add(1, Ordering::SeqCst);
        Self(id)
    }
}

pub struct Window {
    id: WindowID,
    position: Vector2d,
    buffer: BufferCanvas<Vec<u8>>,
    container_size: Size,
}

impl Window {
    fn new(pixel_format: PixelFormat, size: Size, container_size: Size) -> Self {
        Self {
            id: WindowID::generate(),
            position: Vector2d::zero(),
            buffer: BufferCanvas::vec_backed(pixel_format, size),
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
    top_layers: Vec<SharedWindow>,
    size: Size,
}

impl LayerManager {
    fn new(pixel_format: PixelFormat, size: Size) -> Self {
        Self {
            pixel_format,
            layers: vec![],
            top_layers: vec![],
            size,
        }
    }

    pub fn add_top<D: Draw>(&mut self, draw: D) -> Widget<D> {
        let mut window = Window::new(self.pixel_format, draw.size(), self.size);
        draw.draw(&mut window.buffer);
        let shared = SharedWindow::new(window);
        self.top_layers.push(shared.clone());
        Widget::new(shared, draw)
    }

    pub fn add<D: Draw>(&mut self, draw: D) -> Widget<D> {
        let mut window = Window::new(self.pixel_format, draw.size(), self.size);
        draw.draw(&mut window.buffer);
        let shared = SharedWindow::new(window);
        self.layers.push(shared.clone());
        Widget::new(shared, draw)
    }

    pub fn draw_window<C: Canvas>(&self, canvas: &mut C, id: WindowID) -> Option<Rectangle> {
        let mut redraw_area = None;
        for layer in self.layers.iter().chain(self.top_layers.iter()) {
            if layer.id == id {
                let layer = layer.lock();
                let area = Rectangle::new(layer.position.into(), layer.buffer.size());
                canvas.draw_buffer_area(layer.position, &layer.buffer, area);
                redraw_area = Some(area)
            } else if let Some(area) = redraw_area {
                let layer = layer.lock();
                canvas.draw_buffer_area(layer.position, &layer.buffer, area);
            }
        }
        redraw_area
    }

    pub fn draw_area<C: Canvas>(&self, canvas: &mut C, area: Rectangle) {
        for layer in self.layers.iter().chain(self.top_layers.iter()) {
            let layer = layer.lock();
            canvas.draw_buffer_area(layer.position, &layer.buffer, area);
        }
    }
}

impl Draw for LayerManager {
    fn size(&self) -> Size {
        self.size
    }

    fn draw<C: Canvas>(&self, canvas: &mut C) {
        for layer in self.layers.iter().chain(self.top_layers.iter()) {
            let layer = layer.lock();
            canvas.draw_buffer(layer.position, &layer.buffer);
        }
    }
}
