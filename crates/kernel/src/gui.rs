use alloc::{rc::Rc, string::ToString};
use pomelo_common::graphics::GraphicConfig;
use spinning_top::Spinlock;

use crate::{
    graphics::{
        buffer::BufferCanvas,
        canvas::Canvas,
        window_manager::{self, WindowManager, WindowId},
        screen::{self, Screen},
        widgets::{self, console, text_window::TextWindow, Framed},
        Color, Draw, Point, Rectangle, Size, UCoordinate, Vector2d,
    },
    keyboard::KeyCode,
    mouse,
    timer::Timer,
};

pub fn create_gui(graphic_config: &GraphicConfig) -> GUI {
    let mut window_manager = window_manager::create_window_manager(graphic_config);
    let screen = screen::create_screen(graphic_config);
    let size = Size::new(
        graphic_config.horisontal_resolution as UCoordinate,
        graphic_config.vertical_resolution as UCoordinate,
    );
    window_manager
        .add(widgets::Desktop::new(size))
        .set_draggable(false);
    console::register(&mut window_manager);
    mouse::initialize(&mut window_manager);

    let counter = Framed::new("Counter".to_string(), Counter::new());
    let mut counter = window_manager.add(counter);
    counter.move_relative(crate::graphics::Vector2d::new(300, 200));

    let text_field = TextWindow::new(Color::BLACK, Color::WHITE, 30);
    let text_field = Framed::new("Text box".to_string(), text_field);
    let mut text_field = window_manager.add(text_field);
    text_field.move_relative(crate::graphics::Vector2d::new(300, 300));
    let text_field = Rc::new(Spinlock::new(text_field));
    let cloned = text_field.clone();

    let mut timer = Timer::new();

    timer.schedule(500, 500, move || {
        let mut text_field = cloned.lock();
        text_field.draw_mut().draw_mut().flip_cursor_visibility();
        text_field.buffer();
        crate::events::fire_redraw_window(text_field.window_id());
    });

    task_b(&mut window_manager);

    GUI::new(window_manager, screen, timer, counter, text_field)
}

lazy_static! {
    static ref TASK_B_FRAME: Spinlock<Option<widgets::Widget<widgets::Framed<Counter>>>> =
        Spinlock::new(None);
}

fn task_b(window_manager: &mut WindowManager) {
    let frame = Framed::new("Another task".to_string(), Counter::new());
    let mut frame = window_manager.add(frame);
    frame.move_relative(crate::graphics::Vector2d::new(400, 200));
    TASK_B_FRAME.lock().get_or_insert_with(|| frame);
    crate::task::spawn_task(task_b_main, 0);
}

extern "sysv64" fn task_b_main(arg: u64) {
    crate::println!("Task b spawned with arg {}", arg);
    // crate::println!("Task b interrupt flg: {}", x86_64::instructions::interrupts::are_enabled());
    let mut locked = TASK_B_FRAME.lock();
    let frame = locked.as_mut().unwrap();
    loop {
        frame.draw_mut().draw_mut().inc();
        frame.buffer();
        crate::events::fire_redraw_window(frame.window_id());
        x86_64::instructions::interrupts::enable_and_hlt();
    }
}

pub struct Counter(usize);
impl Counter {
    fn new() -> Self {
        Self(0)
    }
    fn inc(&mut self) {
        self.0 += 1;
    }
}
impl Draw for Counter {
    fn size(&self) -> Size {
        Size::new(
            crate::graphics::canvas::GLYPH_WIDTH * 20,
            crate::graphics::canvas::GLYPH_HEIGHT,
        )
    }
    fn draw<C: crate::graphics::canvas::Canvas>(&self, canvas: &mut C) {
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
    counter: widgets::Widget<widgets::Framed<Counter>>,
    text_field: Rc<Spinlock<widgets::Widget<widgets::Framed<TextWindow>>>>,
}

impl GUI {
    fn new(
        window_manager: WindowManager,
        screen: Screen,
        timer: Timer,
        counter: widgets::Widget<widgets::Framed<Counter>>,
        text_field: Rc<Spinlock<widgets::Widget<widgets::Framed<TextWindow>>>>,
    ) -> Self {
        let buffer = BufferCanvas::vec_backed(screen.pixel_format(), screen.size());
        Self {
            window_manager,
            screen,
            buffer,
            timer,
            counter,
            text_field,
        }
    }

    pub fn render(&mut self) {
        self.window_manager.draw(&mut self.buffer);
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
        self.counter.draw_mut().draw_mut().inc();
        self.counter.buffer();
        self.render_window(self.counter.window_id());
    }

    pub fn drag(&mut self, start: Point, end: Point) {
        self.window_manager.drag(start, end);
    }

    pub fn key_press(&mut self, key_code: KeyCode) {
        if let Some(c) = key_code.to_char() {
            let mut locked = self.text_field.lock();
            locked.draw_mut().draw_mut().push(c);
            locked.buffer();
            crate::events::fire_redraw_window(locked.window_id());
        }
    }
}
