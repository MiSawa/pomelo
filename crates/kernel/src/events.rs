use lazy_static::lazy_static;
use spin::Mutex;
use x86_64::instructions::interrupts;

use crate::{prelude::*, ring_buffer::ArrayRingBuffer, xhci};

lazy_static! {
    static ref GLOAL_QUEUE: Mutex<ArrayRingBuffer<Event, 1024>> = Mutex::new(Default::default());
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Event {
    XHCI,
}

pub fn enque(event: Event) {
    interrupts::without_interrupts(|| {
        GLOAL_QUEUE.lock().try_push_back(event).ok();
    })
}

pub fn event_loop() -> Result<!> {
    log::info!("start event loop");
    loop {
        interrupts::disable();
        let mut queue = GLOAL_QUEUE.lock();
        if let Some(event) = queue.pop_front() {
            log::trace!("Got an event {:?}", event);
            match event {
                Event::XHCI => xhci::handle_events(),
            }
        } else {
            drop(queue);
            interrupts::enable_and_hlt();
        }
    }
}
