use super::{
    widgets::Widget,
    window_manager::{WindowId, WindowStateShared},
};
use crate::{
    graphics::{buffer::VecBufferCanvas, Point, Vector2d},
    triple_buffer::Producer,
};

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

    pub fn move_relative(&mut self, v: Vector2d) -> (Point, Point) {
        let ret = self.state.move_relative(v);
        crate::events::fire_redraw();
        ret
    }
}
