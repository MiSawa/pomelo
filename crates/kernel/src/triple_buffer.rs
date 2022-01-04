use core::{
    cell::UnsafeCell,
    sync::atomic::{AtomicU8, Ordering},
};

use alloc::sync::Arc;

type I = u8;
const NEWER_BIT: I = 0x10;
const INDEX_MASK: I = 0x03;
type AtomicI = AtomicU8;

pub struct TripleBuffer<T> {
    producer: Producer<T>,
    consumer: Consumer<T>,
}

impl<T: Default> Default for TripleBuffer<T> {
    fn default() -> Self {
        Self::from_fn(T::default)
    }
}

impl<T> TripleBuffer<T> {
    pub fn new(value: &T) -> Self
    where
        T: Clone,
    {
        Self::from_fn(move || value.clone())
    }
    pub fn from_fn(mut default_value_generator: impl FnMut() -> T) -> Self {
        let buffers = [
            UnsafeCell::new(default_value_generator()),
            UnsafeCell::new(default_value_generator()),
            UnsafeCell::new(default_value_generator()),
        ];
        let free_index: AtomicI = AtomicI::new(2);
        let state = SharedState {
            buffers,
            free_index,
        };
        let state = Arc::new(state);
        Self {
            producer: Producer {
                state: state.clone(),
                write_index: 1,
            },
            consumer: Consumer {
                state,
                read_index: 0,
            },
        }
    }
    pub fn split(self) -> (Producer<T>, Consumer<T>) {
        (self.producer, self.consumer)
    }
}

pub struct Producer<T> {
    state: Arc<SharedState<T>>,
    write_index: I,
}
pub struct Consumer<T> {
    state: Arc<SharedState<T>>,
    read_index: I,
}
impl<T> Producer<T> {
    pub fn current_buffer(&mut self) -> &mut T {
        let ptr = self.state.buffers[self.write_index as usize].get();
        unsafe { &mut *ptr }
    }
    pub fn write(&mut self, value: T) {
        *self.current_buffer() = value;
        self.publish();
    }
    pub fn publish(&mut self) {
        let previous_free = self
            .state
            .free_index
            .swap(self.write_index | NEWER_BIT, Ordering::AcqRel);
        self.write_index = previous_free & INDEX_MASK;
    }
}
impl<T> Consumer<T> {
    pub fn has_update(&self) -> bool {
        self.state.free_index.load(Ordering::Relaxed) & !INDEX_MASK != 0
    }
    pub fn read_last_buffer(&self) -> &T {
        let ptr = self.state.buffers[self.read_index as usize].get();
        unsafe { &*ptr }
    }
    pub fn read(&mut self) -> &T {
        self.update();
        let ptr = self.state.buffers[self.read_index as usize].get();
        unsafe { &*ptr }
    }
    pub fn read_update(&mut self) -> Option<&T> {
        if self.update() {
            let ptr = self.state.buffers[self.read_index as usize].get();
            Some(unsafe { &*ptr })
        } else {
            None
        }
    }
    pub fn update(&mut self) -> bool {
        if !self.has_update() {
            return false;
        }
        let previous_free = self
            .state
            .free_index
            .swap(self.read_index, Ordering::AcqRel);
        self.read_index = previous_free & INDEX_MASK;
        true
    }
}

struct SharedState<T> {
    buffers: [UnsafeCell<T>; 3],
    free_index: AtomicI,
}
unsafe impl<T: Send> Sync for SharedState<T> {}

