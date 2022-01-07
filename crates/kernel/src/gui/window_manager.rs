extern crate alloc;

use core::sync::atomic::{AtomicUsize, Ordering};

use alloc::{boxed::Box, sync::Arc, vec, vec::Vec};
use pomelo_common::graphics::{GraphicConfig, PixelFormat};
use spinning_top::{Spinlock, SpinlockGuard};

use super::{
    widgets::Widget,
    windows::{MoveNeedRedraw, NopWindowEventHandler, Window, WindowEvent, WindowEventHandler},
};
use crate::{
    graphics::{
        buffer::VecBufferCanvas, canvas::Canvas, Point, Rectangle, Size, UCoordinate, Vector2d,
    },
    task::{TaskBuilder, TaskMainWithArg},
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
                draggable: true,
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
    #[must_use]
    pub fn move_relative(&mut self, v: Vector2d) -> MoveNeedRedraw {
        let mut locked = self.locked();
        let old_position = locked.position;
        locked.position =
            (locked.position + v).clamped(Rectangle::new(Point::zero(), locked.screen_size));
        MoveNeedRedraw {
            start_pos: old_position,
            end_pos: locked.position,
        }
    }
}

struct WindowHandle {
    id: WindowId,
    state: WindowStateShared,
    buffer: Consumer<VecBufferCanvas>,
    event_handler: Box<dyn WindowEventHandler>,
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

pub struct TaskedWindowBuilder<W: Widget, E: From<WindowEvent>> {
    window_builder: WindowBuilder<W>,
    task_builder: TaskBuilder<E, Window<W>, !>,
}
impl<W: Widget, E: From<WindowEvent>> TaskedWindowBuilder<W, E> {
    #[must_use]
    pub fn new(name: &'static str, widget: W, task_main: TaskMainWithArg<E, Window<W>>) -> Self {
        Self {
            window_builder: WindowBuilder::new(widget),
            task_builder: crate::task::builder_with_arg(name, task_main),
        }
    }

    #[must_use]
    pub fn configure_window(
        mut self,
        f: impl FnOnce(WindowBuilder<W>) -> WindowBuilder<W>,
    ) -> Self {
        self.window_builder = f(self.window_builder);
        self
    }

    #[must_use]
    pub fn configure_task(
        mut self,
        f: impl FnOnce(TaskBuilder<E, Window<W>, !>) -> TaskBuilder<E, Window<W>, !>,
    ) -> Self {
        self.task_builder = f(self.task_builder);
        self
    }
}

pub struct WindowBuilder<W: Widget> {
    widget: W,
    position: Point,
    draggable: bool,
    top: bool,
}

impl<W: Widget> WindowBuilder<W> {
    #[must_use]
    pub fn new(widget: W) -> Self {
        Self {
            widget,
            position: Point::zero(),
            draggable: true,
            top: false,
        }
    }

    #[must_use]
    pub fn set_position(mut self, position: Point) -> Self {
        self.position = position;
        self
    }

    #[must_use]
    pub fn set_draggable(mut self, draggable: bool) -> Self {
        self.draggable = draggable;
        self
    }

    #[must_use]
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
    focused: Option<WindowId>,
}

impl WindowManager {
    fn new(pixel_format: PixelFormat, size: Size) -> Self {
        Self {
            pixel_format,
            layers: vec![],
            top_layers: vec![],
            size,
            focused: None,
        }
    }

    pub fn pixel_format(&self) -> PixelFormat {
        self.pixel_format
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
                event_handler: Box::new(NopWindowEventHandler),
            },
            Window::new(id, state, producer, widget),
        )
    }

    pub fn create<W: Widget>(&mut self, builder: WindowBuilder<W>) -> Window<W> {
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

    pub fn create_and_spawn<W: Widget, E: 'static + From<WindowEvent>>(
        &mut self,
        builder: TaskedWindowBuilder<W, E>,
    ) {
        let task_builder = builder.task_builder;
        let window_builder = builder.window_builder;
        let (mut window_handle, mut window) = self.create_window(window_builder.widget);
        window.buffer();

        let waking = task_builder.waking();
        let task_handle =
            crate::task::spawn_task(task_builder.set_waking(true).set_arg(Box::new(window)));
        window_handle.event_handler = Box::new(task_handle.clone());

        let mut state = window_handle.state.locked();
        state.position = window_builder.position;
        state.draggable = window_builder.draggable;
        drop(state);
        if waking {
            task_handle.awake();
        }
        if window_builder.top {
            self.top_layers.push(window_handle);
        } else {
            self.layers.push(window_handle);
        }
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
        let mut redraw_area = None;
        let mut window_id_to_focus = None;
        for window in self.layers.iter().chain(self.top_layers.iter()).rev() {
            let mut state = window.state.locked();
            let pos = state.position;
            let rect = Rectangle::new(pos, window.buffer.read_last_buffer().size());
            if state.draggable && rect.contains(&start) {
                let screen_rect = Rectangle::new(Point::zero(), self.size);
                state.position = (state.position + (end - start)).clamped(screen_rect);
                let new_rect = rect + (state.position - pos);
                redraw_area = Some(rect.union(&new_rect));
                window_id_to_focus = Some(window.id);
                break;
            }
        }
        if let Some(new_id) = window_id_to_focus {
            if self.focused != window_id_to_focus {
                if let Some(old_id) = self.focused.take() {
                    if let Some(w) = self
                        .layers
                        .iter_mut()
                        .chain(self.top_layers.iter_mut())
                        .rev()
                        .find(|w| w.id == old_id)
                    {
                        w.event_handler.on_blur();
                    }
                }
            }
            self.focused = window_id_to_focus;
            if let Some(w) = self
                .layers
                .iter_mut()
                .chain(self.top_layers.iter_mut())
                .rev()
                .find(|w| w.id == new_id)
            {
                w.event_handler.on_focus();
            }
            if let Some(i) =
                self.layers
                    .iter()
                    .enumerate()
                    .find_map(|(i, w)| if w.id == new_id { Some(i) } else { None })
            {
                self.layers[i..].rotate_right(1);
            }
        }
        if let Some(area) = redraw_area {
            crate::events::fire_redraw_area(area);
        }
    }

    pub fn render(&mut self, canvas: &mut impl Canvas) {
        for window in self.layers.iter_mut().chain(self.top_layers.iter_mut()) {
            let buffer = window.buffer.read();
            let pos = window.state.position();
            canvas.draw_buffer(pos.into(), buffer);
        }
    }

    fn get_window_handle(&mut self, id: WindowId) -> Option<&mut WindowHandle> {
        self.layers
            .iter_mut()
            .chain(self.top_layers.iter_mut())
            .find(|w| w.id == id)
    }
    pub fn key_press(&mut self, key_code: crate::keyboard::KeyCode) {
        if let Some(id) = self.focused {
            if let Some(handle) = self.get_window_handle(id) {
                handle.event_handler.on_key_press(key_code);
            }
        }
    }
}
