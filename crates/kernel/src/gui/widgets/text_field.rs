use alloc::boxed::Box;

use crate::{
    gui::windows::{Window, WindowEvent},
    task::Receiver,
};

use super::{text_window::TextWindow, Framed, Widget};

#[derive(Clone, Copy, Debug)]
pub enum TextFieldMessage {
    Blink,
    WindowEvent(WindowEvent),
}
impl From<WindowEvent> for TextFieldMessage {
    fn from(e: WindowEvent) -> Self {
        Self::WindowEvent(e)
    }
}

pub extern "sysv64" fn text_field_main(
    mut receiver: Box<Receiver<TextFieldMessage>>,
    mut text_field: Box<Window<Framed<TextWindow>>>,
) {
    crate::timer::schedule(500, 500, receiver.handle(), TextFieldMessage::Blink);
    loop {
        let message = receiver.dequeue_or_wait();
        match message {
            TextFieldMessage::Blink => text_field
                .widget_mut()
                .widget_mut()
                .flip_cursor_visibility(),
            TextFieldMessage::WindowEvent(e) => {
                if let WindowEvent::KeyPress(k) = e {
                    if let Some(c) = k.to_char() {
                        text_field.widget_mut().widget_mut().push(c);
                    }
                }
                text_field.widget_mut().handle_window_event(e);
            }
        }
        text_field.buffer();
        crate::events::fire_redraw_window(text_field.window_id());
    }
}
