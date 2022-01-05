use alloc::{boxed::Box, rc::Rc, string::ToString};
use pomelo_common::graphics::GraphicConfig;
use spinning_top::Spinlock;

use crate::{graphics::{
        buffer::{BufferCanvas, VecBufferCanvas},
        canvas::Canvas,
        screen::{self, Screen},
        Color, Point, Rectangle, Size, UCoordinate, Vector2d,
    }, gui::{
        widgets::{console, text_window::TextWindow, Framed},
        window_manager::{WindowId, WindowManager},
    }, keyboard::KeyCode, task::{Receiver, TaskBuilder, TypedTaskHandle}, timer::Timer};

use self::{
    widgets::{desktop::Desktop, Widget},
    window_manager::WindowBuilder,
    windows::Window,
};

pub mod mouse;
pub mod widgets;
pub mod window_manager;
pub mod windows;

pub const DESKTOP_FG_COLOR: Color = Color::WHITE;
pub const DESKTOP_BG_COLOR: Color = Color::new(45, 118, 237);

pub fn create_gui(graphic_config: &GraphicConfig) -> GUI {
    let mut window_manager = window_manager::create_window_manager(graphic_config);
    let screen = screen::create_screen(graphic_config);
    let size = Size::new(
        graphic_config.horisontal_resolution as UCoordinate,
        graphic_config.vertical_resolution as UCoordinate,
    );
    window_manager.add_builder(WindowBuilder::new(Desktop::new(size)).set_draggable(false));
    console::register(&mut window_manager);
    mouse::initialize(&mut window_manager);

    let counter = Framed::new("Counter".to_string(), Counter::new());
    let counter =
        window_manager.add_builder(WindowBuilder::new(counter).set_position(Point::new(300, 200)));

    let text_field = TextWindow::new(Color::BLACK, Color::WHITE, 30);
    let text_field = Framed::new("Text box".to_string(), text_field);
    let text_field = window_manager
        .add_builder(WindowBuilder::new(text_field).set_position(Point::new(300, 300)));
    let text_field = Rc::new(Spinlock::new(text_field));
    let cloned = text_field.clone();

    let mut timer = Timer::new();

    timer.schedule(500, 500, move || {
        let mut text_field = cloned.lock();
        text_field
            .widget_mut()
            .widget_mut()
            .flip_cursor_visibility();
        text_field.buffer();
        crate::events::fire_redraw_window(text_field.window_id());
    });

    task_b(&mut window_manager);

    crate::task::spawn_task(TaskBuilder::new(idle_task_main)).send(10);
    let idle = crate::task::spawn_task(TaskBuilder::new(idle_task_main));

    GUI::new(window_manager, screen, timer, counter, text_field, idle)
}

lazy_static! {
    static ref TASK_B_FRAME: Spinlock<Option<Window<widgets::Framed<Counter>>>> =
        Spinlock::new(None);
}

extern "sysv64" fn idle_task_main(mut receiver: Box<Receiver<u64>>) {
    // log::warn!("Idle task {}", arg);
    loop {
        let value = receiver.dequeue_or_wait();
        crate::println!("Idle task received {}", value);
    }
}

fn task_b(window_manager: &mut WindowManager) {
    let frame = Framed::new("Another task".to_string(), Counter::new());
    let frame =
        window_manager.add_builder(WindowBuilder::new(frame).set_position(Point::new(400, 200)));
    TASK_B_FRAME.lock().get_or_insert_with(|| frame);
    crate::task::spawn_task(TaskBuilder::new(task_b_main).set_arg(0));
}

extern "sysv64" fn task_b_main(_receiver: Box<Receiver<u64>>) {
    // crate::println!("Task b spawned");
    // crate::println!("Task b interrupt flg: {}", x86_64::instructions::interrupts::are_enabled());
    let mut locked = TASK_B_FRAME.lock();
    let frame = locked.as_mut().unwrap();
    loop {
        frame.widget_mut().widget_mut().inc();
        frame.buffer();
        crate::events::fire_redraw_window(frame.window_id());
        x86_64::instructions::interrupts::enable_and_hlt();
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
    screen: Screen,
    buffer: BufferCanvas<alloc::vec::Vec<u8>>,
    timer: Timer,
    counter: Window<widgets::Framed<Counter>>,
    text_field: Rc<Spinlock<Window<widgets::Framed<TextWindow>>>>,
    idle: TypedTaskHandle<u64>,
}

impl GUI {
    fn new(
        window_manager: WindowManager,
        screen: Screen,
        timer: Timer,
        counter: Window<widgets::Framed<Counter>>,
        text_field: Rc<Spinlock<Window<widgets::Framed<TextWindow>>>>,
        idle: TypedTaskHandle<u64>,
    ) -> Self {
        let buffer = BufferCanvas::vec_backed(screen.pixel_format(), screen.size());
        Self {
            window_manager,
            screen,
            buffer,
            timer,
            counter,
            text_field,
            idle,
        }
    }

    pub fn render(&mut self) {
        self.window_manager.render(&mut self.buffer);
        self.screen.draw_buffer(Vector2d::zero(), &self.buffer);
    }

    pub fn render_window(&mut self, id: WindowId) {
        if let Some(area) = self.window_manager.draw_window(&mut self.buffer, id) {
            self.screen
                .draw_buffer_area(Vector2d::zero(), &self.buffer, area);
        }
    }

    pub fn render_area(&mut self, area: Rectangle) {
        self.window_manager.draw_area(&mut self.buffer, area);
        self.screen
            .draw_buffer_area(Vector2d::zero(), &self.buffer, area);
    }

    pub fn tick(&mut self) {
        self.timer.tick();
        let t = self.counter.widget_mut().widget_mut().inc();
        if t % 1000 == 0 {
            self.idle.send(t as u64);
        }
        self.counter.buffer();
        self.render_window(self.counter.window_id());
    }

    pub fn drag(&mut self, start: Point, end: Point) {
        self.window_manager.drag(start, end);
    }

    pub fn key_press(&mut self, key_code: KeyCode) {
        if let Some(c) = key_code.to_char() {
            let mut locked = self.text_field.lock();
            locked.widget_mut().widget_mut().push(c);
            locked.buffer();
            crate::events::fire_redraw_window(locked.window_id());
        }
    }
}
