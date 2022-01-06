use core::sync::atomic::{AtomicPtr, Ordering};

use alloc::{boxed::Box, collections::VecDeque};
use lazy_static::lazy_static;
use spinning_top::Spinlock;
use x86_64::instructions::interrupts;

use crate::{
    graphics::Rectangle,
    gui::{window_manager::WindowId, GUI},
    keyboard::{self, KeyCode},
    prelude::*,
    task::{spawn_task, Receiver, TaskBuilder, TypedTaskHandle},
    xhci,
};

lazy_static! {
    static ref XHCI_HANDLE: AtomicPtr<TypedTaskHandle<u8>> = AtomicPtr::default();
    static ref GUI_HANDLE: AtomicPtr<TypedTaskHandle<Event>> = AtomicPtr::default();
    static ref REDRAW_QUEUE: Spinlock<VecDeque<Event>> = Spinlock::new(VecDeque::new());
}
const MAX_PENDING_REDRAW: usize = 10;

pub fn initialize() -> Receiver<Event> {
    let (receiver, queue) = crate::task::initialize::<Event>();
    GUI_HANDLE.store(Box::into_raw(Box::new(queue)), Ordering::Release);
    let handle = spawn_task(
        TaskBuilder::new("xhci", xhci_handler)
            .set_priority(10)
            .set_stack_size(10 * 1024 * 1024),
    );
    XHCI_HANDLE.store(Box::into_raw(Box::new(handle)), Ordering::Release);
    receiver
}

extern "sysv64" fn xhci_handler(mut receiver: Box<Receiver<u8>>) {
    loop {
        x86_64::instructions::interrupts::enable();
        let _ = receiver.dequeue_or_wait();
        log::info!("Got a XHCI event");
        xhci::handle_events();
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Event {
    Drag { start: Point, end: Point },
    KeyPress(KeyCode),
    Redraw,
    RedrawWindow(WindowId),
    RedrawArea(Rectangle),
}

fn with_handle<E, F: FnOnce(TypedTaskHandle<E>)>(handle: &AtomicPtr<TypedTaskHandle<E>>, f: F) {
    let handle = handle.load(Ordering::Acquire);
    if handle.is_null() {
        return;
    }
    f(unsafe { (*handle).clone() });
}

fn with_draw_queue_locked<T, F: FnOnce(spinning_top::SpinlockGuard<VecDeque<Event>>) -> T>(
    f: F,
) -> T {
    interrupts::without_interrupts(|| {
        let locked = REDRAW_QUEUE.lock();
        f(locked)
    })
}

pub fn fire_xhci() {
    with_handle(&XHCI_HANDLE, |q| q.send(0))
}

pub fn fire_drag(start: Point, end: Point) {
    with_handle(&GUI_HANDLE, |q| q.send(Event::Drag { start, end }));
}

pub fn fire_key_press(keycode: keyboard::KeyCode) {
    with_handle(&GUI_HANDLE, |q| q.send(Event::KeyPress(keycode)));
}

pub fn fire_redraw() {
    with_draw_queue_locked(|mut q| {
        q.clear();
        q.push_back(Event::Redraw)
    });
    with_handle(&GUI_HANDLE, |q| q.awake());
}
pub fn fire_redraw_window(id: WindowId) {
    with_draw_queue_locked(|mut q| {
        if q.len() < MAX_PENDING_REDRAW {
            q.push_back(Event::RedrawWindow(id))
        } else {
            q.clear();
        }
    });
    with_handle(&GUI_HANDLE, |q| q.awake());
}
pub fn fire_redraw_area(area: Rectangle) {
    with_draw_queue_locked(|mut q| {
        if q.len() < MAX_PENDING_REDRAW {
            q.push_back(Event::RedrawArea(area));
        } else {
            q.clear();
        }
    });
    with_handle(&GUI_HANDLE, |q| q.awake());
}

pub fn event_loop(mut gui: GUI) -> Result<!> {
    // crate::task::current_task().set_priority(3);
    log::info!("Start event loop");
    loop {
        let state = gui.event_receiver.handle().load_state();
        let event = gui
            .event_receiver
            .try_dequeue()
            .or_else(|| with_draw_queue_locked(|mut q| q.pop_front()));
        let event = if let Some(event) = event {
            event
        } else {
            gui.event_receiver.handle().try_compare_and_sleep(state);
            continue;
        };
        log::info!("Got an event {:?}", event);
        match event {
            Event::Drag { start, end } => {
                gui.drag(start, end);
            }
            Event::KeyPress(keycode) => {
                gui.key_press(keycode);
            }
            Event::Redraw => {
                with_draw_queue_locked(|mut q| q.clear());
                gui.render();
            }
            Event::RedrawWindow(id) => {
                gui.render_window(id);
            }
            Event::RedrawArea(area) => {
                gui.render_area(area);
            }
        }
    }
}
