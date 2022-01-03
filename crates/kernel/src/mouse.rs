use bitflags::bitflags;
use spinning_top::Spinlock;

use crate::{
    graphics::{canvas::Canvas, layer::LayerManager, widgets::Widget, Color, Draw},
    prelude::*,
};

lazy_static! {
    static ref MOUSE_CURSOR: Spinlock<Option<MouseCursor>> = Spinlock::new(Option::None);
}

const HEIGHT: usize = 24;
const WIDTH: usize = 15;
const TRANSPARENT_COLOR: Color = Color::new(1, 2, 3);

pub fn initialize(layer_manager: &mut LayerManager) {
    MOUSE_CURSOR.lock().get_or_insert_with(|| {
        let cursor = MouseCursorImage;
        let mut widget = layer_manager.add_top(cursor);
        widget.set_draggable(false);
        widget.move_relative(Vector2d::new(200, 200));
        widget.set_transparent_color(Some(TRANSPARENT_COLOR));
        MouseCursor {
            widget,
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
    widget: Widget<MouseCursorImage>,
    buttons: MouseButtons,
}

impl MouseCursor {
    fn handle_event(&mut self, buttons: MouseButtons, v: Vector2d) {
        let (start, end) = self.widget.move_relative(v);
        if self.buttons.contains(MouseButtons::LEFT) && buttons.contains(MouseButtons::LEFT) {
            crate::events::fire_drag(start, end);
        }
        self.buttons = buttons;
    }
}

struct MouseCursorImage;

impl Draw for MouseCursorImage {
    fn size(&self) -> Size {
        Size::new(WIDTH as UCoordinate, HEIGHT as UCoordinate)
    }

    fn draw<C: Canvas>(&self, canvas: &mut C) {
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
