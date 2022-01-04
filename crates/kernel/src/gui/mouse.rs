use bitflags::bitflags;
use spinning_top::Spinlock;

use crate::{
    graphics::{buffer::VecBufferCanvas, canvas::Canvas, Color},
    gui::{widgets::Widget, window_manager::WindowManager},
    prelude::*,
};

use super::{window_manager::WindowBuilder, windows::Window};

lazy_static! {
    static ref MOUSE_CURSOR: Spinlock<Option<MouseCursor>> = Spinlock::new(Option::None);
}

const HEIGHT: usize = 24;
const WIDTH: usize = 15;
const TRANSPARENT_COLOR: Color = Color::new(1, 2, 3);

pub fn initialize(window_manager: &mut WindowManager) {
    MOUSE_CURSOR.lock().get_or_insert_with(|| {
        let cursor = MouseCursorImage;
        let window = window_manager.add_builder(
            WindowBuilder::new(cursor)
                .set_draggable(false)
                .set_position(Point::new(200, 200)),
        );
        MouseCursor {
            window,
            buttons: MouseButtons::empty(),
        }
    });
}

pub extern "C" fn observe_cursor_move(button_state: u8, x: i8, y: i8) {
    log::trace!("Mouse event!");
    let mut cursor = MOUSE_CURSOR.lock();
    if let Some(cursor) = cursor.as_mut() {
        let buttons = MouseButtons::from_bits_truncate(button_state);
        cursor.handle_event(buttons, Vector2d::new(x as ICoordinate, y as ICoordinate));
    }
}

bitflags! {
    struct MouseButtons: u8 {
        const LEFT   = 0b001;
        const RIGHT  = 0b010;
        const MIDDLE = 0b100;
    }
}

struct MouseCursor {
    window: Window<MouseCursorImage>,
    buttons: MouseButtons,
}

impl MouseCursor {
    fn handle_event(&mut self, buttons: MouseButtons, v: Vector2d) {
        let (start, end) = self.window.move_relative(v);
        if self.buttons.contains(MouseButtons::LEFT) && buttons.contains(MouseButtons::LEFT) {
            crate::events::fire_drag(start, end);
        }
        self.buttons = buttons;
    }
}

struct MouseCursorImage;

impl Widget for MouseCursorImage {
    fn render(&self, canvas: &mut VecBufferCanvas) {
        canvas.set_transparent_color(Some(TRANSPARENT_COLOR));
        canvas.resize(Size::new(WIDTH as UCoordinate, HEIGHT as UCoordinate));
        for (y, row) in MOUSE_CURSOR_SHAPE.iter().enumerate() {
            for (x, cell) in row.iter().enumerate() {
                let color = match cell {
                    b'@' => Color::BLACK,
                    b'.' => Color::WHITE,
                    _ => TRANSPARENT_COLOR,
                };
                canvas.draw_pixel(color, Point::new(x as ICoordinate, y as ICoordinate));
            }
        }
    }
}

const MOUSE_CURSOR_SHAPE: [[u8; WIDTH]; HEIGHT] = [
    *b"@              ",
    *b"@@             ",
    *b"@.@            ",
    *b"@..@           ",
    *b"@...@          ",
    *b"@....@         ",
    *b"@.....@        ",
    *b"@......@       ",
    *b"@.......@      ",
    *b"@........@     ",
    *b"@.........@    ",
    *b"@..........@   ",
    *b"@...........@  ",
    *b"@............@ ",
    *b"@......@@@@@@@@",
    *b"@......@       ",
    *b"@....@@.@      ",
    *b"@...@ @.@      ",
    *b"@..@   @.@     ",
    *b"@.@    @.@     ",
    *b"@@      @.@    ",
    *b"@       @.@    ",
    *b"         @.@   ",
    *b"         @@@   ",
];
