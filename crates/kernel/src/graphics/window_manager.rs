extern crate alloc;

use core::sync::atomic::{AtomicUsize, Ordering};

use alloc::{sync::Arc, vec, vec::Vec};
use pomelo_common::graphics::{GraphicConfig, PixelFormat};
use spinning_top::{Spinlock, SpinlockGuard};

use crate::{
    graphics::{
        buffer::BufferCanvas, canvas::Canvas, widgets::Widget, Color, Draw, ICoordinate, Point,
        Rectangle, Size, UCoordinate, Vector2d,
    },
    triple_buffer::{Consumer, Producer, TripleBuffer},
};

pub fn create_window_manager(graphic_config: &GraphicConfig) -> WindowManager {
    WindowManager::new(
        graphic_config.pixel_format,
        Size::new(
            graphic_config.horisontal_resolution as UCoordinate,
            graphic_config.vertical_resolution as UCoordinate,
        ),
    )
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct WindowId(usize);
impl WindowId {
    pub fn generate() -> Self {
        static WINDOW_ID_GENERATOR: AtomicUsize = AtomicUsize::new(0);
        let id = WINDOW_ID_GENERATOR.fetch_add(1, Ordering::SeqCst);
        Self(id)
    }
}

#[derive(Debug)]
pub struct WindowState {
    id: WindowId,
    position: Point,
    draggable: bool,
}

#[derive(Clone, Debug)]
pub struct WindowStateShared {
    inner: Arc<Spinlock<WindowState>>,
}
impl WindowStateShared {
    fn new() -> Self {
        Self {
            inner: Arc::new(Spinlock::new(WindowState {
                id: WindowId::generate(),
                position: Point::zero(),
                draggable: false,
            })),
        }
    }
    fn locked(&self) -> SpinlockGuard<WindowState> {
        self.inner.lock()
    }
    fn position(&self) -> Point {
        self.locked().position
    }
}

struct WindowHandle {
    id: WindowId,
    state: WindowStateShared,
    buffer: Consumer<BufferCanvas<Vec<u8>>>,
}

pub enum MaybeRegistered<D: Draw> {
    Unregistered(D),
    Registered(Widget<D>),
    // Ugggh I want replace_with
    Registering,
}

impl<D: Draw> MaybeRegistered<D> {
    pub fn register_once(&mut self, window_manager: &mut WindowManager) -> &mut Widget<D> {
        let took = core::mem::replace(self, Self::Registering);
        *self = match took {
            MaybeRegistered::Unregistered(d) => Self::Registered(window_manager.add(d)),
            other => other,
        };
        if let MaybeRegistered::Registered(w) = self {
            w
        } else {
            panic!("Whaaat, maybe it failed to register itself?")
        }
    }
    pub fn unwrap_ref(&self) -> &D {
        match self {
            MaybeRegistered::Unregistered(d) => d,
            MaybeRegistered::Registered(w) => w.draw_ref(),
            _ => panic!("Whaaat, maybe it failed to register itself?"),
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

pub struct WindowManager {
    pixel_format: PixelFormat,
    layers: Vec<WindowHandle>,
    top_layers: Vec<WindowHandle>,
    focused: Option<WindowId>,
    size: Size,
}

impl WindowManager {
    fn new(pixel_format: PixelFormat, size: Size) -> Self {
        Self {
            pixel_format,
            layers: vec![],
            top_layers: vec![],
            focused: None,
            size,
        }
    }

    fn create_window(&self, size: Size) -> (WindowHandle, Window) {
        let state = WindowStateShared::new();
        let (producer, consumer) =
            TripleBuffer::from_fn(|| BufferCanvas::vec_backed(self.pixel_format, size)).split();
        let id = state.locked().id;
        (
            WindowHandle {
                id,
                state,
                buffer: consumer,
            },
            Window {
                id,
                state,
                buffer: producer,
            },
        )
    }

    pub fn add_top<D: Draw>(&mut self, draw: D) -> Widget<D> {
        let (handle, window) = self.create_window(draw.size());
        self.top_layers.push(handle);
        Widget::new(window, draw)
    }

    pub fn add<D: Draw>(&mut self, draw: D) -> Widget<D> {
        let (handle, window) = self.create_window(draw.size());
        self.layers.push(handle);
        Widget::new(window, draw)
    }

    pub fn draw_window<C: Canvas>(&self, canvas: &mut C, id: WindowId) -> Option<Rectangle> {
        let mut redraw_area = None;
        for window in self.layers.iter().chain(self.top_layers.iter()) {
            if window.id == id {
                let buffer = window.buffer.read();
                let pos = window.state.locked().position;
                let area = Rectangle::new(pos, buffer.size());
                canvas.draw_buffer_area(pos.into(), buffer, area);
                redraw_area = Some(area)
            } else if let Some(area) = redraw_area {
                let buffer = window.buffer.read();
                let pos = window.state.position();
                canvas.draw_buffer_area(pos.into(), buffer, area);
            }
        }
        redraw_area
    }

    pub fn draw_area<C: Canvas>(&self, canvas: &mut C, area: Rectangle) {
        for window in self.layers.iter().chain(self.top_layers.iter()) {
            let buffer = window.buffer.read();
            let pos = window.state.position();
            canvas.draw_buffer_area(pos.into(), buffer, area);
        }
    }

    pub fn drag(&mut self, start: Point, end: Point) {
        for window in self.layers.iter().chain(self.top_layers.iter()).rev() {
            let mut state = window.state.locked();
            let pos = state.position;
            let rect = Rectangle::new(pos, window.buffer.read_last_buffer().size());
            if state.draggable && rect.contains(&start) {
                let screen_rect = self.bounding_box();
                state.position = (state.position + (end - start)).clamped(screen_rect);
                let new_rect = rect + (state.position - pos);
                crate::events::fire_redraw_area(rect.union(&new_rect));
                break;
            }
        }
    }
}

impl Draw for WindowManager {
    fn size(&self) -> Size {
        self.size
    }

    fn draw<C: Canvas>(&self, canvas: &mut C) {
        for window in self.layers.iter().chain(self.top_layers.iter()) {
            let buffer = window.buffer.read();
            let pos = window.state.position();
            canvas.draw_buffer(pos.into(), buffer);
        }
    }
}
