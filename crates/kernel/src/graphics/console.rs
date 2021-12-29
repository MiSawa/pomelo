const ROWS: usize = 25;
const COLUMNS: usize = 80;

use arrayvec::{ArrayString, ArrayVec};
use pomelo_common::GraphicConfig;
use spin::Mutex;

use crate::graphics::{
    self,
    canvas::{Canvas, GLYPH_HEIGHT, GLYPH_WIDTH},
    screen::{self, Screen},
    Color, ICoordinate, Point,
};

lazy_static! {
    static ref GLOBAL_CONSOLE: Mutex<Option<Console<Screen>>> = Mutex::new(Option::None);
}

pub fn initialize(graphic_config: &GraphicConfig) {
    screen::initialize(graphic_config);
    GLOBAL_CONSOLE.lock().get_or_insert_with(|| {
        Console::new(
            screen::screen(),
            graphics::DESKTOP_FG_COLOR,
            graphics::DESKTOP_BG_COLOR,
        )
    });
}

pub fn global_console() -> impl core::fmt::Write {
    struct GlobalConsoleWrite;
    impl core::fmt::Write for GlobalConsoleWrite {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            let mut console = GLOBAL_CONSOLE.lock();
            let console = console
                .as_mut()
                .expect("Global console should be initialized");
            console.write_str(s)
        }
    }
    GlobalConsoleWrite
}

pub struct Console<C: Canvas> {
    canvas: C,
    buffer: ArrayVec<ArrayString<{ COLUMNS * 4 }>, ROWS>,
    foreground: Color,
    background: Color,
    cursor_point: Point,
}

impl<C: Canvas> Console<C> {
    pub fn new(canvas: C, foreground: Color, background: Color) -> Self {
        let mut buffer = ArrayVec::new();
        buffer.push(ArrayString::new());
        Self {
            buffer,
            canvas,
            foreground,
            background,
            cursor_point: Point::zero(),
        }
    }

    fn rc_to_point(row: usize, column: usize) -> Point {
        Point::new(
            column as ICoordinate * GLYPH_WIDTH as ICoordinate,
            row as ICoordinate * GLYPH_HEIGHT as ICoordinate,
        )
    }

    fn new_line(&mut self) {
        // TODO: Use buffer as a ring buffer.
        if self.buffer.is_full() {
            self.canvas
                .fill_rectangle(self.background, &self.canvas.bounding_box());
            self.buffer.remove(0);
            for (i, row) in self.buffer.iter().enumerate() {
                self.canvas
                    .draw_string(self.foreground, Self::rc_to_point(i, 0), row);
            }
        }
        self.buffer.push(ArrayString::new());
        self.cursor_point = Self::rc_to_point(self.buffer.len() - 1, 0);
    }

    pub fn write_string(&mut self, s: &str) {
        for c in s.chars() {
            if c == '\n' {
                self.new_line();
            } else {
                self.cursor_point.x +=
                    self.canvas.draw_char(self.foreground, self.cursor_point, c) as ICoordinate;
                self.buffer.last_mut().unwrap().try_push(c).ok();
            }
        }
    }
}

impl<C: Canvas> core::fmt::Write for Console<C> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.write_string(s);
        Ok(())
    }
}
