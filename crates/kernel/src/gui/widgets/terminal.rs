use alloc::{
    boxed::Box,
    collections::VecDeque,
    string::{String, ToString},
};
use pomelo_common::graphics::PixelFormat;

use crate::{
    graphics::{
        buffer::VecBufferCanvas,
        canvas::{Canvas, GLYPH_HEIGHT, GLYPH_WIDTH},
        Color, ICoordinate, Point, Rectangle, Size, UCoordinate, Vector2d,
    },
    gui::{
        window_manager::{TaskedWindowBuilder, WindowManager},
        windows::{Window, WindowEvent},
    },
    task::Receiver,
};

use super::{Framed, Widget};

const FG_COLOR: Color = Color::WHITE;
const BG_COLOR: Color = Color::BLACK;
const GLYPH_SIZE: Size = Size::new(GLYPH_WIDTH, GLYPH_HEIGHT);

pub fn create_terminal(wm: &mut WindowManager) {
    let terminal = Framed::new(
        "Terminal".to_string(),
        Terminal::new(wm.pixel_format(), 60, 16),
    );
    wm.create_and_spawn(
        TaskedWindowBuilder::new("terminal", terminal, terminal_main)
            .configure_window(|w| w.set_position(Point::new(300, 100))),
    );
}

#[derive(Clone, Copy, Debug)]
pub enum TerminalMessage {
    WindowEvent(WindowEvent),
    Blink,
}
impl From<WindowEvent> for TerminalMessage {
    fn from(e: WindowEvent) -> Self {
        Self::WindowEvent(e)
    }
}

struct Terminal {
    cols: usize,
    rows: usize,
    buffers: VecDeque<VecBufferCanvas>,
    current_string: String,
    cursor_line: usize,
    cursor_visible: bool,
    focused: bool,
    dirty: bool,
}
impl Terminal {
    fn new(pixel_format: PixelFormat, cols: usize, rows: usize) -> Self {
        let buffers = core::iter::repeat_with(|| {
            let size = Size::new(cols as UCoordinate * GLYPH_WIDTH, GLYPH_HEIGHT);
            let mut buf = VecBufferCanvas::vec_backed(pixel_format, size);
            buf.fill_rectangle(BG_COLOR, Rectangle::new(Point::zero(), size));
            buf
        })
        .take(rows)
        .collect();
        Self {
            cols,
            rows,
            buffers,
            current_string: String::new(),
            cursor_line: 0,
            cursor_visible: true,
            focused: true,
            dirty: true,
        }
    }

    fn col_to_point(c: usize) -> Point {
        Point::new(c as ICoordinate * GLYPH_WIDTH as ICoordinate, 0)
    }

    fn flip_cursor_visibility(&mut self) {
        self.cursor_visible ^= true;
        let color = if self.cursor_visible {
            FG_COLOR
        } else {
            BG_COLOR
        };
        self.buffers[self.cursor_line].fill_rectangle(
            color,
            Rectangle::new(Self::col_to_point(self.current_string.len()), GLYPH_SIZE),
        );
        self.dirty = true;
    }

    fn new_line(&mut self) {
        // Erase cursor on the last line
        if self.cursor_visible {
            self.flip_cursor_visibility();
        }
        if self.cursor_line + 1 == self.buffers.len() {
            self.buffers.rotate_left(1);
            self.buffers.back_mut().unwrap().fill_rectangle(
                BG_COLOR,
                Rectangle::new(
                    Point::new(0, 0),
                    Size::new(UCoordinate::MAX, UCoordinate::MAX),
                ),
            );
            self.dirty = true;
        } else {
            self.cursor_line += 1;
        }
        self.current_string.clear();
    }

    fn push_char(&mut self, c: char) {
        if c == '\n' {
            self.new_line();
        } else if c == '\x08' {
            if self.current_string.pop().is_some() {
                // Delete the char as well as the cursor
                self.buffers[self.cursor_line].fill_rectangle(
                    BG_COLOR,
                    Rectangle::new(
                        Self::col_to_point(self.current_string.len()),
                        Size::new(GLYPH_WIDTH * 2, GLYPH_HEIGHT),
                    ),
                );
            }
        } else {
            // Delete cursor
            self.buffers[self.cursor_line].fill_rectangle(
                BG_COLOR,
                Rectangle::new(Self::col_to_point(self.current_string.len()), GLYPH_SIZE),
            );
            self.buffers[self.cursor_line].draw_char(
                FG_COLOR,
                Self::col_to_point(self.current_string.len()),
                c,
            );
            self.current_string.push(c);
        }
        self.dirty = true;
    }
}

impl Widget for Terminal {
    fn on_focus(&mut self) {
        self.focused = true;
        self.dirty = true;
    }

    fn on_blur(&mut self) {
        self.focused = false;
        self.dirty = true;
    }

    fn on_key_press(&mut self, key_code: crate::keyboard::KeyCode) {
        if let Some(c) = key_code.to_char() {
            self.push_char(c);
        }
    }

    fn render(&self, canvas: &mut crate::graphics::buffer::VecBufferCanvas) {
        canvas.resize(Size::new(
            self.cols as UCoordinate * GLYPH_WIDTH,
            self.rows as UCoordinate * GLYPH_HEIGHT,
        ));
        let mut y = 0;
        for b in self.buffers.iter() {
            canvas.draw_buffer(Vector2d::new(0, y), b);
            y += GLYPH_HEIGHT as ICoordinate;
        }
    }
}

extern "sysv64" fn terminal_main(
    mut receiver: Box<Receiver<TerminalMessage>>,
    mut window: Box<Window<Framed<Terminal>>>,
) {
    crate::timer::schedule(500, 500, receiver.handle(), TerminalMessage::Blink);
    loop {
        let message = receiver.dequeue_or_wait();
        let terminal = window.widget_mut().widget_mut();
        match message {
            TerminalMessage::WindowEvent(e) => {
                window.widget_mut().handle_window_event(e);
            }
            TerminalMessage::Blink => {
                if terminal.focused {
                    terminal.flip_cursor_visibility();
                }
            }
        }
        let terminal = window.widget_mut().widget_mut();
        if terminal.dirty {
            terminal.dirty = false;
            window.buffer();
            crate::events::fire_redraw_window(window.window_id());
        }
    }
}
