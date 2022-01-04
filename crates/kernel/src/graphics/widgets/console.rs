use arrayvec::ArrayString;

use pomelo_common::graphics::GraphicConfig;
use spinning_top::Spinlock;

use crate::{
    graphics::{
        self,
        canvas::{Canvas, GLYPH_HEIGHT, GLYPH_WIDTH},
        layer::{LayerManager, MaybeRegistered},
        screen::{self, Screen},
        Color, Draw, ICoordinate, Point, Rectangle, Size, UCoordinate, Vector2d,
    },
    ring_buffer::ArrayRingBuffer,
};

const ROWS: usize = 40;
const COLUMNS: usize = 160;

lazy_static! {
    static ref GLOBAL_CONSOLE: Spinlock<Console> = Spinlock::new(Console::new(
        graphics::DESKTOP_FG_COLOR,
        graphics::DESKTOP_BG_COLOR
    ));
}
lazy_static! {
    static ref FALLBACK_CONSOLE: Spinlock<Option<(Console, Screen)>> = Spinlock::new(None);
}

pub fn initialize(graphic_config: &GraphicConfig) {
    FALLBACK_CONSOLE.lock().get_or_insert_with(|| {
        (
            Console::new(Color::WHITE, Color::BLACK),
            screen::create_screen(graphic_config),
        )
    });
}
pub fn register(layer_manager: &mut LayerManager) {
    let mut console = GLOBAL_CONSOLE.lock();
    console.register(layer_manager);
}

pub fn fallback_console() -> impl core::fmt::Write {
    struct FallbackConsoleWrite;
    impl core::fmt::Write for FallbackConsoleWrite {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            if let Some((console, screen)) = FALLBACK_CONSOLE.lock().as_mut() {
                console.write_str(s);
                console.draw(screen);
                Ok(())
            } else {
                Err(core::fmt::Error)
            }
        }
    }
    FallbackConsoleWrite
}

pub fn global_console() -> impl core::fmt::Write {
    struct GlobalConsoleWrite;
    impl core::fmt::Write for GlobalConsoleWrite {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            let mut console = GLOBAL_CONSOLE.lock();
            console.write_str(s);
            Ok(())
        }
    }
    impl Drop for GlobalConsoleWrite {
        fn drop(&mut self) {
            crate::events::fire_redraw();
        }
    }
    GlobalConsoleWrite
}

struct Row {
    text: ArrayString<{ COLUMNS * 4 }>,
    dirty: bool,
    foreground: Color,
    background: Color,
}
impl Row {
    fn new(foreground: Color, background: Color) -> Self {
        Self {
            text: ArrayString::new(),
            dirty: true,
            foreground,
            background,
        }
    }

    fn clear(&mut self) {
        self.text.clear();
        self.dirty = true;
    }

    fn push_char(&mut self, c: char) {
        self.text.try_push(c).ok();
        self.dirty = true;
    }

    fn pop_char(&mut self) -> Option<char> {
        let ret = self.text.pop();
        self.dirty = true;
        ret
    }
}
impl Draw for Row {
    fn size(&self) -> Size {
        Size::new(COLUMNS as UCoordinate * GLYPH_WIDTH, GLYPH_HEIGHT)
    }

    fn draw<C: Canvas>(&self, canvas: &mut C) {
        canvas.fill_rectangle(self.background, self.bounding_box());
        canvas.draw_string(self.foreground, Point::zero(), self.text.as_str());
    }
}

struct Console {
    rows: ArrayRingBuffer<MaybeRegistered<Row>, ROWS>,
    current_row: usize,
}

impl Console {
    fn new(foreground: Color, background: Color) -> Self {
        let mut rows = ArrayRingBuffer::new();
        for _ in 0..ROWS {
            rows.push_back(MaybeRegistered::Unregistered(Row::new(
                foreground, background,
            )));
        }
        Self {
            rows,
            current_row: 0,
        }
    }

    fn register(&mut self, layer_manager: &mut LayerManager) {
        for (i, row) in self.rows.iter_mut().enumerate() {
            let widget = row.register_once(layer_manager);
            widget.move_relative(Vector2d::new(
                0,
                GLYPH_HEIGHT as ICoordinate * i as ICoordinate,
            ));
            widget.set_draggable(false);
        }
    }

    fn eol(&mut self) {
        if self.current_row + 1 == self.rows.len() {
            // Pop the top row, clear the text
            let mut prev = self.rows.pop_front().unwrap();
            let row = prev.unwrap_mut();
            row.clear();
            // Shift the top row down
            if let Some(w) = prev.get_widget() {
                w.move_relative(Vector2d::new(
                    0,
                    GLYPH_HEIGHT as ICoordinate * (ROWS - 1) as ICoordinate,
                ));
            }
            // Shift other rows up
            for row in self.rows.iter_mut() {
                if let Some(w) = row.get_widget() {
                    w.move_relative(Vector2d::new(0, -(GLYPH_HEIGHT as ICoordinate)));
                }
            }
            // Put the row as the bottom row
            self.rows.push_back(prev);
        } else {
            self.current_row += 1;
        }
    }

    fn write_str(&mut self, s: &str) {
        for c in s.chars() {
            if c == '\n' {
                self.eol();
            } else if c == '\x08' {
                let row = &mut self.rows[self.current_row];
                let row = row.unwrap_mut();
                if row.pop_char().is_none() && self.current_row > 0 {
                    self.current_row -= 1
                }
            } else {
                let row = &mut self.rows[self.current_row];
                let row = row.unwrap_mut();
                row.push_char(c);
            }
        }
        for i in (0..=self.current_row).rev() {
            let w = &mut self.rows[i];
            let row = w.unwrap_mut();
            if row.dirty {
                row.dirty = false;
                w.buffer();
            } else {
                break;
            }
        }
    }
}

impl Draw for Console {
    fn size(&self) -> Size {
        Size::new(
            COLUMNS as UCoordinate * GLYPH_WIDTH,
            ROWS as UCoordinate * GLYPH_HEIGHT,
        )
    }

    fn draw<C: Canvas>(&self, canvas: &mut C) {
        for (i, r) in self.rows.iter().enumerate() {
            let d = r.unwrap_ref();
            canvas.fill_rectangle(
                d.background,
                Rectangle::new(
                    Point::new(0, i as ICoordinate * GLYPH_HEIGHT as ICoordinate),
                    Size::new(COLUMNS as UCoordinate * GLYPH_WIDTH, GLYPH_HEIGHT),
                ),
            );
            canvas.draw_string(
                d.foreground,
                Point::new(0, i as ICoordinate * GLYPH_HEIGHT as ICoordinate),
                &d.text,
            );
        }
    }
}
