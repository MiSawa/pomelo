use crate::graphics::{canvas::Canvas, Color, ICoordinate, Point};

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

pub fn render_mouse_cursor(canvas: &mut impl Canvas, p: Point) {
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
