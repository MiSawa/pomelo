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
        Terminal::new(wm.pixel_format(), 16, 60),
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
    rows: usize,
    cols: usize,
    buffers: VecDeque<VecBufferCanvas>,
    current_string: String,
    cursor_row: usize,
    cursor_col: usize,
    cursor_visible: bool,
    focused: bool,
    dirty: bool,
}
impl Terminal {
    fn new(pixel_format: PixelFormat, rows: usize, cols: usize) -> Self {
        let buffers = core::iter::repeat_with(|| {
            let size = Size::new(cols as UCoordinate * GLYPH_WIDTH, GLYPH_HEIGHT);
            let mut buf = VecBufferCanvas::vec_backed(pixel_format, size);
            buf.fill_rectangle(BG_COLOR, Rectangle::new(Point::zero(), size));
            buf
        })
        .take(rows)
        .collect();
        let mut this = Self {
            rows,
            cols,
            buffers,
            current_string: String::new(),
            cursor_row: 0,
            cursor_col: 0,
            cursor_visible: true,
            focused: true,
            dirty: true,
        };
        this.prompt();
        this
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
        self.buffers[self.cursor_row].fill_rectangle(
            color,
            Rectangle::new(Self::col_to_point(self.cursor_col), GLYPH_SIZE),
        );
        self.dirty = true;
    }

    fn new_line(&mut self) {
        // Erase cursor on the last line
        if self.cursor_visible {
            self.flip_cursor_visibility();
        }
        if self.cursor_row + 1 == self.buffers.len() {
            self.buffers.rotate_left(1);
            let last = self.buffers.back_mut().unwrap();
            last.fill_rectangle(BG_COLOR, last.bounding_box());
            self.dirty = true;
        } else {
            self.cursor_row += 1;
        }
        self.cursor_col = 0;
    }

    fn push_char_impl(&mut self, c: char, command_related: bool) {
        if c == '\n' {
            self.new_line();
            if command_related {
                let command = core::mem::take(&mut self.current_string);
                self.execute_command(command);
                self.prompt();
            }
        } else if c == '\x08' {
            if self.current_string.pop().is_some() {
                // Delete cursor
                self.buffers[self.cursor_row].fill_rectangle(
                    BG_COLOR,
                    Rectangle::new(Self::col_to_point(self.cursor_col), GLYPH_SIZE),
                );
                if self.cursor_col == 0 {
                    if self.cursor_row == 0 {
                        // Happens when you filled chars until it scrolls, and then go back to the top.
                        // Just ignore it.
                    } else {
                        self.cursor_row -= 1;
                        self.cursor_col = self.cols - 1;
                    }
                } else {
                    self.cursor_col -= 1;
                }
                // Delete the char
                self.buffers[self.cursor_row].fill_rectangle(
                    BG_COLOR,
                    Rectangle::new(Self::col_to_point(self.cursor_col), GLYPH_SIZE),
                );
            }
        } else {
            // Delete cursor
            self.buffers[self.cursor_row].fill_rectangle(
                BG_COLOR,
                Rectangle::new(Self::col_to_point(self.cursor_col), GLYPH_SIZE),
            );
            // Write char
            self.buffers[self.cursor_row].draw_char(
                FG_COLOR,
                Self::col_to_point(self.cursor_col),
                c,
            );
            self.cursor_col += 1;
            if self.cursor_col == self.cols {
                self.new_line();
            }
            if command_related {
                self.current_string.push(c);
            }
        }
        self.dirty = true;
    }

    fn push_char(&mut self, c: char) {
        self.push_char_impl(c, true);
    }

    fn prompt(&mut self) {
        self.push_char_impl('>', false);
    }

    fn as_result_writer(&mut self) -> impl '_ + core::fmt::Write {
        struct W<'a>(&'a mut Terminal);
        impl<'a> core::fmt::Write for W<'a> {
            fn write_str(&mut self, s: &str) -> core::fmt::Result {
                for c in s.chars() {
                    self.0.push_char_impl(c, false);
                }
                Ok(())
            }
        }
        W(self)
    }

    fn execute_command(&mut self, command: String) {
        if let Some((command, args)) = command.split_once(' ') {
            if command == "echo" {
                for c in args.chars() {
                    self.push_char_impl(c, false);
                }
                self.new_line();
            } else {
                for c in "Unknown command".chars() {
                    self.push_char_impl(c, false);
                }
                self.new_line();
            }
        } else if command == "clear" {
            self.cursor_row = 0;
            self.cursor_col = 0;
            for buffer in self.buffers.iter_mut() {
                buffer.fill_rectangle(BG_COLOR, buffer.bounding_box());
            }
        } else if command == "lspci" {
            use core::fmt::Write;
            for device in crate::pci::scan_devices() {
                for func in device.scan_functions() {
                    let (base, sub, interface) = func.class().to_code();
                    writeln!(
                        self.as_result_writer(),
                        "{:02x}:{:02x}.{} vend={:04x} head={:02x} class={:02x}.{:02x}.{:02x}",
                        func.bus(),
                        func.device(),
                        func.function(),
                        func.vendor_id(),
                        func.header_type(),
                        base,
                        sub,
                        interface
                    )
                    .ok();
                }
            }
        } else {
            for c in "Unknown command".chars() {
                self.push_char_impl(c, false);
            }
            self.new_line();
        }
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
