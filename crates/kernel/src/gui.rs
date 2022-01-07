use alloc::{boxed::Box, string::ToString};
use pomelo_common::graphics::GraphicConfig;

use crate::{
    events::Event,
    graphics::{
        buffer::{BufferCanvas, VecBufferCanvas},
        canvas::Canvas,
        screen::{self, Screen},
        Color, Point, Rectangle, Size, UCoordinate, Vector2d,
    },
    gui::{
        widgets::{console, text_window::TextWindow, Framed},
        window_manager::{WindowId, WindowManager},
    },
    keyboard::KeyCode,
    task::{self, Receiver},
};

use self::{
    widgets::{desktop::Desktop, Widget},
    window_manager::{TaskedWindowBuilder, WindowBuilder},
    windows::{Window, WindowEvent},
};

pub mod mouse;
pub mod widgets;
pub mod window_manager;
pub mod windows;

pub const DESKTOP_FG_COLOR: Color = Color::WHITE;
pub const DESKTOP_BG_COLOR: Color = Color::new(45, 118, 237);

pub fn create_gui(graphic_config: &GraphicConfig) -> GUI {
    let event_receiver = crate::events::initialize();

    let mut window_manager = window_manager::create_window_manager(graphic_config);
    let screen = screen::create_screen(graphic_config);
    let size = Size::new(
        graphic_config.horisontal_resolution as UCoordinate,
        graphic_config.vertical_resolution as UCoordinate,
    );
    window_manager.create(WindowBuilder::new(Desktop::new(size)).set_draggable(false));
    console::register(&mut window_manager);
    mouse::initialize(&mut window_manager);

    let counter = Framed::new("Counter".to_string(), Counter::new());
    let counter =
        window_manager.create(WindowBuilder::new(counter).set_position(Point::new(300, 200)));

    create_text_field(&mut window_manager);

    GUI::new(window_manager, event_receiver, screen, counter)
}

#[derive(Clone, Copy, Debug)]
enum TextFieldMessage {
    Blink,
    WindowEvent(WindowEvent),
}
impl From<WindowEvent> for TextFieldMessage {
    fn from(e: WindowEvent) -> Self {
        Self::WindowEvent(e)
    }
}
fn create_text_field(wm: &mut WindowManager) {
    let text_field = Framed::new(
        "Text box".to_string(),
        TextWindow::new(Color::BLACK, Color::WHITE, 30),
    );
    wm.create_and_spawn(
        TaskedWindowBuilder::new("text_field", text_field, text_field_main)
            .configure_window(|w| w.set_position(Point::new(300, 300))),
    );
}
extern "sysv64" fn text_field_main(
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

const TRANSPARENT_COLOR: Color = Color::new(1, 2, 3);
pub struct Counter(usize);
impl Counter {
    fn new() -> Self {
        Self(0)
    }
    fn inc(&mut self) -> usize {
        self.0 += 1;
        self.0
    }
}
impl Widget for Counter {
    fn render(&self, canvas: &mut VecBufferCanvas) {
        let size = Size::new(
            crate::graphics::canvas::GLYPH_WIDTH * 20,
            crate::graphics::canvas::GLYPH_HEIGHT,
        );
        canvas.resize(size);
        canvas.set_transparent_color(Some(TRANSPARENT_COLOR));
        canvas.fill_rectangle(TRANSPARENT_COLOR, Rectangle::new(Point::zero(), size));
        canvas
            .draw_fmt(Color::BLACK, Point::zero(), format_args!("{:010}", self.0))
            .ok();
        canvas
            .draw_fmt(Color::BLACK, Point::zero(), format_args!("{:010}", self.0))
            .ok();
    }
}

pub struct GUI {
    window_manager: WindowManager,
    pub event_receiver: Receiver<Event>,
    screen: Screen,
    buffer: BufferCanvas<alloc::vec::Vec<u8>>,
    counter: Window<widgets::Framed<Counter>>,
}

impl GUI {
    fn new(
        window_manager: WindowManager,
        event_receiver: Receiver<Event>,
        screen: Screen,
        counter: Window<widgets::Framed<Counter>>,
    ) -> Self {
        let buffer = BufferCanvas::vec_backed(screen.pixel_format(), screen.size());
        Self {
            window_manager,
            event_receiver,
            screen,
            buffer,
            counter,
        }
    }

    fn inc_counter(&mut self) {
        self.counter.widget_mut().widget_mut().inc();
        self.counter.buffer();
    }

    pub fn render(&mut self) {
        self.inc_counter();
        self.window_manager.render(&mut self.buffer);
        self.screen.draw_buffer(Vector2d::zero(), &self.buffer);
    }

    pub fn render_window(&mut self, id: WindowId) {
        self.inc_counter();
        if let Some(area) = self.window_manager.draw_window(&mut self.buffer, id) {
            self.screen
                .draw_buffer_area(Vector2d::zero(), &self.buffer, area);
        }
    }

    pub fn render_area(&mut self, area: Rectangle) {
        self.inc_counter();
        self.window_manager.draw_area(&mut self.buffer, area);
        self.screen
            .draw_buffer_area(Vector2d::zero(), &self.buffer, area);
    }

    pub fn drag(&mut self, start: Point, end: Point) {
        self.inc_counter();
        self.window_manager.drag(start, end);
    }

    pub fn key_press(&mut self, key_code: KeyCode) {
        self.inc_counter();
        self.window_manager.key_press(key_code);
    }
}
