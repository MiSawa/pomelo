use super::{
    widgets::Widget,
    window_manager::{WindowId, WindowStateShared},
};
use crate::{
    graphics::{buffer::VecBufferCanvas, Point, Rectangle, Size, Vector2d},
    keyboard::KeyCode,
    task::TypedTaskHandle,
    triple_buffer::Producer,
};

pub struct MoveNeedRedraw {
    pub(crate) start_pos: Point,
    pub(crate) end_pos: Point,
}
impl MoveNeedRedraw {
    pub fn redraw_with_size(self, size: Size) {
        let start_rectangle = Rectangle::new(self.start_pos, size);
        let end_rectangle = Rectangle::new(self.end_pos, size);
        crate::events::fire_redraw_area(start_rectangle.union(&end_rectangle));
    }
    pub fn redraw_all(self) {
        crate::events::fire_redraw();
    }
}

#[derive(Clone, Copy, Debug)]
pub enum WindowEvent {
    Focus,
    Blur,
    KeyPress(KeyCode),
}

pub trait WindowEventHandler {
    fn on_focus(&mut self) {}
    fn on_blur(&mut self) {}
    fn on_key_press(&mut self, _key: KeyCode) {}
    fn handle_window_event(&mut self, e: WindowEvent) {
        match e {
            WindowEvent::Focus => self.on_focus(),
            WindowEvent::Blur => self.on_blur(),
            WindowEvent::KeyPress(k) => self.on_key_press(k),
        }
    }
}

pub struct NopWindowEventHandler;
impl WindowEventHandler for NopWindowEventHandler {}

impl<E: From<WindowEvent>> WindowEventHandler for TypedTaskHandle<E> {
    fn on_focus(&mut self) {
        self.send(WindowEvent::Focus.into())
    }

    fn on_blur(&mut self) {
        self.send(WindowEvent::Blur.into())
    }

    fn on_key_press(&mut self, key: KeyCode) {
        self.send(WindowEvent::KeyPress(key).into());
    }
}

pub struct Window<W: Widget> {
    id: WindowId,
    state: WindowStateShared,
    buffer: Producer<VecBufferCanvas>,
    widget: W,
}

impl<W: Widget> Window<W> {
    pub fn new(
        id: WindowId,
        state: WindowStateShared,
        buffer: Producer<VecBufferCanvas>,
        widget: W,
    ) -> Self {
        Self {
            id,
            state,
            buffer,
            widget,
        }
    }

    pub fn window_id(&self) -> WindowId {
        self.id
    }

    pub fn buffer(&mut self) {
        self.widget.render(self.buffer.current_buffer());
        self.buffer.publish();
    }

    pub fn widget_ref(&self) -> &W {
        &self.widget
    }

    pub fn widget_mut(&mut self) -> &mut W {
        &mut self.widget
    }

    #[must_use]
    pub fn move_relative(&mut self, v: Vector2d) -> MoveNeedRedraw {
        self.state.move_relative(v)
    }
}
