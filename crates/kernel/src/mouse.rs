use pomelo_common::GraphicConfig;
use spin::Mutex;

use crate::{
    graphic::{canvas::Canvas, screen, Color, DESKTOP_BG_COLOR},
    prelude::*,
};

lazy_static! {
    static ref MOUSE_CURSOR: Mutex<Option<MouseCursor>> = Mutex::new(Option::None);
}

pub fn initialize(graphic_config: &GraphicConfig) {
    screen::initialize(graphic_config);
    let screen_size = screen::screen().size();
    MOUSE_CURSOR.lock().get_or_insert_with(|| {
        let mut cursor = MouseCursor::new(Point::new(100, 100), screen_size);
        cursor.move_relative(Vector2d::zero());
        cursor
    });
}

pub extern "C" fn observe_cursor_move(x: i8, y: i8) {
    let mut cursor = MOUSE_CURSOR.lock();
    let cursor = cursor.as_mut().expect("Mouse cursor should be initialized");
    cursor.move_relative(Vector2d::new(x as ICoordinate, y as ICoordinate));
}

struct MouseCursor {
    position: Point,
    screen_size: Size,
}
impl MouseCursor {
    fn new(position: Point, screen_size: Size) -> Self {
        Self {
            position,
            screen_size,
        }
    }

    fn move_relative(&mut self, v: Vector2d) {
        let screen = screen::screen();
        erase_mouse_cursor(&mut screen.lock(), self.position);
        self.position += v;
        self.position.x = self.position.x.clamp(0, self.screen_size.x as ICoordinate);
        self.position.y = self.position.y.clamp(0, self.screen_size.y as ICoordinate);
        render_mouse_cursor(&mut screen.lock(), self.position);
    }
}

fn render_mouse_cursor(canvas: &mut impl Canvas, p: Point) {
    for (y, row) in MOUSE_CURSOR_SHAPE.iter().enumerate() {
        for (x, cell) in row.iter().enumerate() {
            let color = match cell {
                b'@' => Color::BLACK,
                b'.' => Color::WHITE,
                _ => continue,
            };
            canvas.draw_pixel(
                color,
                Point::new(p.x + x as ICoordinate, p.y + y as ICoordinate),
            );
        }
    }
}

fn erase_mouse_cursor(canvas: &mut impl Canvas, p: Point) {
    for (y, row) in MOUSE_CURSOR_SHAPE.iter().enumerate() {
        for (x, cell) in row.iter().enumerate() {
            let color = match cell {
                b'@' | b'.' => DESKTOP_BG_COLOR,
                _ => continue,
            };
            canvas.draw_pixel(
                color,
                Point::new(p.x + x as ICoordinate, p.y + y as ICoordinate),
            );
        }
    }
}

const MOUSE_CURSOR_SHAPE: [[u8; 15]; 24] = [
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
