use pomelo_common::GraphicConfig;
use spin::Mutex;

use crate::{
    graphics::{canvas::Canvas, screen},
    prelude::*,
};

lazy_static! {
    static ref MOUSE_CURSOR: Mutex<Option<MouseCursor>> = Mutex::new(Option::None);
}

pub fn initialize(graphic_config: &GraphicConfig) {
    screen::initialize(graphic_config);
    let screen_size = screen::screen().size();
    MOUSE_CURSOR
        .lock()
        .get_or_insert_with(|| MouseCursor::new(Point::new(100, 100), screen_size));
}

pub extern "C" fn observe_cursor_move(a: i8, b: i8) {
    let mut cursor = MOUSE_CURSOR.lock();
    let _cursor = cursor.as_mut().expect("Mouse cursor should be initialized");
    println!("{}, {}", a, b);
}

struct MouseCursor {
    position: Point,
    screen_size: Size,
}
impl MouseCursor {
    const fn new(position: Point, screen_size: Size) -> Self {
        Self {
            position,
            screen_size,
        }
    }
}
