use spinning_top::Spinlock;

use crate::{
    graphics::{canvas::Canvas, layer::LayerManager, widgets::Widget, Color, Draw},
    prelude::*,
};

lazy_static! {
    static ref MOUSE_CURSOR: Spinlock<Option<Widget<MouseCursor>>> = Spinlock::new(Option::None);
}

const HEIGHT: usize = 24;
const WIDTH: usize = 15;
const TRANSPARENT_COLOR: Color = Color::new(1, 2, 3);

pub fn initialize(layer_manager: &mut LayerManager) {
    MOUSE_CURSOR.lock().get_or_insert_with(|| {
        let cursor = MouseCursor;
        let mut widget = layer_manager.add(cursor);
        widget.move_relative(Vector2d::new(200, 200));
        widget.set_transparent_color(Some(TRANSPARENT_COLOR));
        widget
    });
}

pub extern "C" fn observe_cursor_move(x: i8, y: i8) {
    log::info!("Mouse event!");
    let mut cursor = MOUSE_CURSOR.lock();
    if let Some(cursor) = cursor.as_mut() {
        cursor.move_relative(Vector2d::new(x as ICoordinate, y as ICoordinate));
    }
}

struct MouseCursor;

impl Draw for MouseCursor {
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
