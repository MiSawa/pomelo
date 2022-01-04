extern crate alloc;

use core::sync::atomic::{AtomicUsize, Ordering};

use alloc::{sync::Arc, vec, vec::Vec};
use pomelo_common::graphics::{GraphicConfig, PixelFormat};
use spinning_top::{Spinlock, SpinlockGuard};

use super::{widgets::Widget, windows::Window};
use crate::{
    graphics::{
        buffer::VecBufferCanvas, canvas::Canvas, Point, Rectangle, Size,
        UCoordinate, Vector2d,
    },
    triple_buffer::{Consumer, TripleBuffer},
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
    screen_size: Size,
}

#[derive(Clone, Debug)]
pub struct WindowStateShared {
    inner: Arc<Spinlock<WindowState>>,
}
impl WindowStateShared {
    fn new(screen_size: Size) -> Self {
        Self {
            inner: Arc::new(Spinlock::new(WindowState {
                id: WindowId::generate(),
                position: Point::zero(),
                draggable: false,
                screen_size,
            })),
        }
    }
    fn locked(&self) -> SpinlockGuard<WindowState> {
        self.inner.lock()
    }
    fn position(&self) -> Point {
        self.locked().position
    }
    pub fn move_relative(&mut self, v: Vector2d) -> (Point, Point) {
        let mut locked = self.locked();
        let old_position = locked.position;
        locked.position =
            (locked.position + v).clamped(Rectangle::new(Point::zero(), locked.screen_size));
        (old_position, locked.position)
    }
}

struct WindowHandle {
    id: WindowId,
    state: WindowStateShared,
    buffer: Consumer<VecBufferCanvas>,
}

pub enum MaybeRegistered<W: Widget> {
    Unregistered(W),
    Registered(Window<W>),
    // Ugggh I want replace_with
    Registering,
}

impl<W: Widget> MaybeRegistered<W> {
    pub fn register_once_with(
        &mut self,
        window_manager: &mut WindowManager,
        f: impl FnOnce(&mut WindowManager, W) -> Window<W>,
    ) -> &mut Window<W> {
        let took = core::mem::replace(self, Self::Registering);
        *self = match took {
            MaybeRegistered::Unregistered(w) => Self::Registered(f(window_manager, w)),
            other => other,
        };
        if let MaybeRegistered::Registered(w) = self {
            w
        } else {
            panic!("Whaaat, maybe it failed to register itself?")
        }
    }
    pub fn unwrap_ref(&self) -> &W {
        match self {
            MaybeRegistered::Unregistered(w) => w,
            MaybeRegistered::Registered(w) => w.widget_ref(),
            _ => panic!("Whaaat, maybe it failed to register itself?"),
        }
    }
    pub fn unwrap_mut(&mut self) -> &mut W {
        match self {
            MaybeRegistered::Unregistered(w) => w,
            MaybeRegistered::Registered(w) => w.widget_mut(),
            _ => panic!("Whaaat, maybe it failed to register itself?"),
        }
    }
    pub fn buffer(&mut self) {
        if let MaybeRegistered::Registered(w) = self {
            w.buffer();
        }
    }
    pub fn get_window(&mut self) -> Option<&mut Window<W>> {
        if let MaybeRegistered::Registered(w) = self {
            Some(w)
        } else {
            None
        }
    }
}

pub struct WindowBuilder<W: Widget> {
    widget: W,
    position: Point,
    draggable: bool,
    top: bool,
}

impl<W: Widget> WindowBuilder<W> {
    pub fn new(widget: W) -> Self {
        Self {
            widget,
            position: Point::zero(),
            draggable: false,
            top: false,
        }
    }

    pub fn set_position(mut self, position: Point) -> Self {
        self.position = position;
        self
    }

    pub fn set_draggable(mut self, draggable: bool) -> Self {
        self.draggable = draggable;
        self
    }

    pub fn set_top(mut self, top: bool) -> Self {
        self.top = top;
        self
    }
}

pub struct WindowManager {
    pixel_format: PixelFormat,
    layers: Vec<WindowHandle>,
    top_layers: Vec<WindowHandle>,
    size: Size,
}

impl WindowManager {
    fn new(pixel_format: PixelFormat, size: Size) -> Self {
        Self {
            pixel_format,
            layers: vec![],
            top_layers: vec![],
            size,
        }
    }

    fn create_window<W: Widget>(&self, widget: W) -> (WindowHandle, Window<W>) {
        let state = WindowStateShared::new(self.size);
        let (producer, consumer) =
            TripleBuffer::from_fn(|| VecBufferCanvas::empty(self.pixel_format)).split();
        let id = state.locked().id;
        (
            WindowHandle {
                id,
                state: state.clone(),
                buffer: consumer,
            },
            Window::new(id, state, producer, widget),
        )
    }

    pub fn add_builder<W: Widget>(&mut self, builder: WindowBuilder<W>) -> Window<W> {
        let (handle, mut window) = self.create_window(builder.widget);
        let mut state = handle.state.locked();
        state.position = builder.position;
        state.draggable = builder.draggable;
        drop(state);
        if builder.top {
            self.top_layers.push(handle);
        } else {
            self.layers.push(handle);
        }
        window.buffer();
        window
    }

    pub fn add<W: Widget>(&mut self, widget: W) -> Window<W> {
        let (handle, mut window) = self.create_window(widget);
        self.layers.push(handle);
        window.buffer();
        window
    }

    pub fn draw_window<C: Canvas>(&mut self, canvas: &mut C, id: WindowId) -> Option<Rectangle> {
        let mut redraw_area = None;
        for window in self.layers.iter_mut().chain(self.top_layers.iter_mut()) {
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

    pub fn draw_area<C: Canvas>(&mut self, canvas: &mut C, area: Rectangle) {
        for window in self.layers.iter_mut().chain(self.top_layers.iter_mut()) {
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
                let screen_rect = Rectangle::new(Point::zero(), self.size);
                state.position = (state.position + (end - start)).clamped(screen_rect);
                let new_rect = rect + (state.position - pos);
                crate::events::fire_redraw_area(rect.union(&new_rect));
                break;
            }
        }
    }

    pub fn render(&mut self, canvas: &mut impl Canvas) {
        for window in self.layers.iter_mut().chain(self.top_layers.iter_mut()) {
            let buffer = window.buffer.read();
            let pos = window.state.position();
            canvas.draw_buffer(pos.into(), buffer);
        }
    }
}
