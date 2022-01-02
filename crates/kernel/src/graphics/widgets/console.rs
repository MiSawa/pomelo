use arrayvec::ArrayString;

use spinning_top::Spinlock;

use crate::{
    graphics::{
        self,
        canvas::{Canvas, GLYPH_HEIGHT, GLYPH_WIDTH},
        layer::{LayerManager, MaybeRegistered},
        Color, Draw, ICoordinate, Point, Size, UCoordinate,
    },
    ring_buffer::ArrayRingBuffer,
};

const ROWS: usize = 40;
const COLUMNS: usize = 160;

lazy_static! {
    static ref GLOBAL_CONSOLE: Spinlock<MaybeRegistered<Console>> =
        Spinlock::new(MaybeRegistered::Unregistered(Console::new(
            graphics::DESKTOP_FG_COLOR,
            graphics::DESKTOP_BG_COLOR
        )));
}

pub fn register(layer_manager: &mut LayerManager) {
    let mut console = GLOBAL_CONSOLE.lock();
    console.register_once(layer_manager);
}

pub fn global_console() -> impl core::fmt::Write {
    struct GlobalConsoleWrite;
    impl core::fmt::Write for GlobalConsoleWrite {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            let mut widget = GLOBAL_CONSOLE.lock();
            let console = widget.unwrap_mut();
            console.write_str(s)
        }
    }
    impl Drop for GlobalConsoleWrite {
        fn drop(&mut self) {
            let mut widget = GLOBAL_CONSOLE.lock();
            widget.refresh();
            crate::events::fire_redraw();
        }
    }
    GlobalConsoleWrite
}

struct Console {
    buffer: ArrayRingBuffer<ArrayString<{ COLUMNS * 4 }>, ROWS>,
    foreground: Color,
    background: Color,
}

impl Console {
    fn new(foreground: Color, background: Color) -> Self {
        let mut buffer = ArrayRingBuffer::new();
        buffer.push_back(ArrayString::new());
        Self {
            buffer,
            foreground,
            background,
        }
    }

    fn new_line(&mut self) {
        if self.buffer.is_full() {
            let mut prev = self.buffer.pop_front().unwrap();
            prev.clear();
            self.buffer.push_back(prev);
        } else {
            self.buffer.push_back(ArrayString::new());
        }
    }

    fn write_string(&mut self, s: &str) {
        for c in s.chars() {
            if c == '\n' {
                self.new_line();
            } else {
                if self.buffer.back().map(|s| s.len()).unwrap_or(0) >= COLUMNS {
                    self.new_line();
                    self.buffer.back_mut().unwrap().try_push('>').ok();
                }
                self.buffer.back_mut().unwrap().try_push(c).ok();
            }
        }
    }
}

impl Draw for Console {
    fn size(&self) -> graphics::Size {
        Size::new(
            COLUMNS as UCoordinate * GLYPH_WIDTH,
            ROWS as UCoordinate * GLYPH_HEIGHT,
        )
    }

    fn draw<C: Canvas>(&self, canvas: &mut C) {
        canvas.fill_rectangle(self.background, &self.bounding_box());
        let mut y = 0;
        for s in self.buffer.iter() {
            canvas.draw_string(self.foreground, Point::new(0, y), s);
            y += GLYPH_HEIGHT as ICoordinate;
        }
    }
}

impl core::fmt::Write for Console {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.write_string(s);
        Ok(())
    }
}
