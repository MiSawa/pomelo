use core::sync::atomic::{AtomicUsize, Ordering};
use lazy_static::lazy_static;
use spinning_top::Spinlock;
use x86_64::instructions::interrupts;

use crate::{gui::GUI, prelude::*, ring_buffer::ArrayRingBuffer, xhci};

lazy_static! {
    static ref GLOAL_QUEUE: Spinlock<ArrayRingBuffer<Event, 1024>> =
        Spinlock::new(Default::default());
}
static REDRAW_GENERATION: AtomicUsize = AtomicUsize::new(0);

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Event {
    XHCI,
    REDRAW(usize),
}

fn enque(event: Event) {
    interrupts::without_interrupts(|| {
        GLOAL_QUEUE.lock().try_push_back(event).ok();
    })
}

pub fn fire_xhci() {
    enque(Event::XHCI);
}

pub fn fire_redraw() {
    enque(Event::REDRAW(REDRAW_GENERATION.load(Ordering::SeqCst)));
}

pub fn event_loop(mut gui: GUI) -> Result<!> {
    log::info!("start event loop");
    loop {
        interrupts::disable();
        let mut queue = GLOAL_QUEUE.lock();
        if let Some(event) = queue.pop_front() {
            drop(queue);
            log::trace!("Got an event {:?}", event);
            match event {
                Event::XHCI => xhci::handle_events(),
                Event::REDRAW(v) => {
                    let update =
                        REDRAW_GENERATION.fetch_update(Ordering::SeqCst, Ordering::Relaxed, |g| {
                            if g > v {
                                None
                            } else {
                                Some(g + 1)
                            }
                        });
                    if update.is_ok() {
                        gui.render();
                    }
                }
            }
        } else {
            drop(queue);
            interrupts::enable_and_hlt();
        }
    }
}
