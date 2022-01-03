use core::{cmp::Reverse, sync::atomic::AtomicU64};

use alloc::{boxed::Box, collections::binary_heap::BinaryHeap};

use crate::interrupts::InterruptIndex;

const MAX_TIMER_COUNT: u32 = 10000000;

const LVT_TIMER_ADDRESS: *mut u32 = 0xFEE00320 as *mut u32;
const DIVIDE_CONFIGURATION_ADDRESS: *mut u32 = 0xFEE003E0 as *mut u32;
const INITIAL_COUNT_ADDRESS: *mut u32 = 0xFEE00380 as *mut u32;
// const CURRENT_COUNT_ADDRESS: *const u32 = 0xFEE00390 as *const u32;

fn initialize() {
    const DIVIDE_1_1: u32 = 0b1011;
    const PERIODIC_INTERRUPT: u32 = 0b10 << 16;
    const VECTOR: u32 = InterruptIndex::LAPICTimer as u32;

    unsafe {
        core::ptr::write(DIVIDE_CONFIGURATION_ADDRESS, DIVIDE_1_1);
        core::ptr::write(LVT_TIMER_ADDRESS, PERIODIC_INTERRUPT | VECTOR);
        core::ptr::write(INITIAL_COUNT_ADDRESS, MAX_TIMER_COUNT);
    }
}

struct Task {
    target_tick: u64,
    task_id: usize,
    task: Box<dyn FnMut()>,
}
impl PartialEq for Task {
    fn eq(&self, other: &Self) -> bool {
        (self.target_tick, self.task_id).eq(&(other.target_tick, other.task_id))
    }
}
impl Eq for Task {}
impl PartialOrd for Task {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Task {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        (self.target_tick, self.task_id).cmp(&(other.target_tick, other.task_id))
    }
}

pub(crate) struct Timer {
    queue: BinaryHeap<Reverse<Task>>,
    next_task_id: usize,
    tick: u64,
}

impl Timer {
    pub fn new() -> Self {
        initialize(); // I guess it's not really good to do... but fine.
        Self {
            queue: BinaryHeap::new(),
            next_task_id: 0,
            tick: 0,
        }
    }

    pub fn get_tick(&self) -> u64 {
        self.tick
    }

    pub fn tick(&mut self) {
        self.tick += 1;
        while let Some(Reverse(task)) = self.queue.peek() {
            if task.target_tick > self.tick {
                break;
            }
            let Reverse(mut task) = self.queue.pop().unwrap();
            (task.task)();
        }
    }

    pub fn register<F: 'static + FnOnce()>(&mut self, delay: u64, f: F) {
        let mut opt = Some(f);
        self.queue.push(Reverse(Task {
            target_tick: self.tick + delay,
            task_id: self.next_task_id,
            task: Box::new(move || {
                if let Some(f) = opt.take() {
                    f();
                }
            }),
        }));
        self.next_task_id += 1;
    }
}
