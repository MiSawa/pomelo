use alloc::collections::VecDeque;
use lazy_static::lazy_static;
use spinning_top::Spinlock;
use x86_64::instructions::interrupts;

use crate::{
    graphics::{layer::WindowID, Rectangle},
    gui::GUI,
    keyboard::{self, KeyCode},
    prelude::*,
    xhci,
};

lazy_static! {
    static ref GLOAL_QUEUE: Spinlock<EventQueue> = Spinlock::new(Default::default());
}
const MAX_CHUNKED_REDRAW_COUNT: usize = 10;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Event {
    XHCI,
    LAPICTimer,
    Drag { start: Point, end: Point },
    KeyPress(KeyCode),
    Redraw,
    RedrawWindow(WindowID),
    RedrawArea(Rectangle),
}

#[derive(Default)]
struct EventQueue {
    timer_events: VecDeque<Event>,
    input_event: VecDeque<Event>,
    xhci_events: VecDeque<Event>,
    redraw_events: VecDeque<Event>,
}

fn with_queue_locked<T, F: FnOnce(spinning_top::SpinlockGuard<EventQueue>) -> T>(f: F) -> T {
    interrupts::without_interrupts(|| {
        let locked = GLOAL_QUEUE.lock();
        f(locked)
    })
}

pub fn fire_xhci() {
    with_queue_locked(|mut q| q.xhci_events.push_back(Event::XHCI));
}

pub fn fire_lapic_timer() {
    with_queue_locked(|mut q| q.timer_events.push_back(Event::LAPICTimer));
}

pub fn fire_drag(start: Point, end: Point) {
    with_queue_locked(|mut q| q.input_event.push_back(Event::Drag { start, end }));
}

pub fn fire_key_press(keycode: keyboard::KeyCode) {
    with_queue_locked(|mut q| q.input_event.push_back(Event::KeyPress(keycode)));
}

pub fn fire_redraw() {
    with_queue_locked(|mut q| {
        q.redraw_events.clear();
        q.redraw_events.push_back(Event::Redraw);
    });
}
pub fn fire_redraw_window(id: WindowID) {
    with_queue_locked(|mut q| {
        if q.redraw_events.len() > MAX_CHUNKED_REDRAW_COUNT {
            q.redraw_events.clear();
            q.redraw_events.push_back(Event::Redraw);
        } else {
            q.redraw_events.push_back(Event::RedrawWindow(id));
        }
    });
}
pub fn fire_redraw_area(area: Rectangle) {
    with_queue_locked(|mut q| {
        if q.redraw_events.len() > MAX_CHUNKED_REDRAW_COUNT {
            q.redraw_events.clear();
            q.redraw_events.push_back(Event::Redraw);
        } else {
            q.redraw_events.push_back(Event::RedrawArea(area));
        }
    });
}

fn deque() -> Option<Event> {
    with_queue_locked(|mut q| {
        if let Some(ret) = q.timer_events.pop_front() {
            return Some(ret);
        }
        if let Some(ret) = q.input_event.pop_front() {
            return Some(ret);
        }
        if let Some(ret) = q.xhci_events.pop_front() {
            return Some(ret);
        }
        if let Some(ret) = q.redraw_events.pop_front() {
            return Some(ret);
        }
        None
    })
}

pub fn event_loop(mut gui: GUI) -> Result<!> {
    log::info!("start event loop");
    loop {
        interrupts::disable();
        if let Some(event) = deque() {
            interrupts::enable();
            log::trace!("Got an event {:?}", event);
            match event {
                Event::LAPICTimer => {
                    gui.tick();
                }
                Event::XHCI => {
                    xhci::handle_events();
                }
                Event::Drag { start, end } => {
                    gui.drag(start, end);
                }
                Event::KeyPress(keycode) => {
                    gui.key_press(keycode);
                }
                Event::Redraw => {
                    gui.render();
                }
                Event::RedrawWindow(id) => {
                    gui.render_window(id);
                }
                Event::RedrawArea(area) => {
                    gui.render_area(area);
                }
            }
        } else {
            // interrupts::enable_and_hlt();
            interrupts::enable();
            crate::task::try_switch_context();
        }
    }
}
