use core::{
    cell::UnsafeCell,
    sync::atomic::{AtomicPtr, AtomicUsize, Ordering},
};

use alloc::{boxed::Box, sync::Arc};

pub struct MPSCConsumer<T> {
    state: Arc<SharedState<T>>,
}
impl<T> Default for MPSCConsumer<T> {
    fn default() -> Self {
        Self::new()
    }
}
impl<T> MPSCConsumer<T> {
    pub fn new() -> Self {
        Self {
            state: Arc::new(SharedState::new()),
        }
    }
    pub fn dequeue(&mut self) -> Option<T> {
        self.state.dequeue()
    }
    pub fn approximate_len(&self) -> usize {
        self.state.len.load(Ordering::Relaxed)
    }
    pub fn producer(&self) -> MPSCProducer<T> {
        MPSCProducer {
            state: self.state.clone(),
        }
    }
}

pub struct MPSCProducer<T> {
    state: Arc<SharedState<T>>,
}
impl<T> Clone for MPSCProducer<T> {
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
        }
    }
}
impl<T> MPSCProducer<T> {
    pub fn enqueue(&self, value: T) {
        self.state.enqueue(value);
    }
}

struct Node<T> {
    next: AtomicPtr<Node<T>>,
    value: Option<T>,
}
impl<T> Node<T> {
    fn dummy() -> Self {
        Self {
            next: AtomicPtr::default(),
            value: None,
        }
    }
    fn new(value: T) -> Self {
        Self {
            next: AtomicPtr::default(),
            value: Some(value),
        }
    }
    fn into_ptr(self) -> *mut Self {
        Box::into_raw(Box::new(self))
    }
}
struct SharedState<T> {
    len: AtomicUsize,
    head: AtomicPtr<Node<T>>,
    tail: UnsafeCell<*mut Node<T>>,
}

impl<T> SharedState<T> {
    fn new() -> Self {
        let dummy = Node::dummy().into_ptr();
        Self {
            len: AtomicUsize::new(0),
            head: AtomicPtr::new(dummy),
            tail: UnsafeCell::new(dummy),
        }
    }

    fn enqueue(&self, value: T) {
        let node = Node::new(value).into_ptr();
        let previous_head = self.head.swap(node, Ordering::AcqRel);
        unsafe { (*previous_head).next.store(node, Ordering::Release) };
        self.len.fetch_add(1, Ordering::Release);
    }

    fn dequeue(&self) -> Option<T> {
        let tail = unsafe { *self.tail.get() };
        let next = unsafe { (*tail).next.load(Ordering::Acquire) };
        if next.is_null() {
            return None;
        }
        unsafe { *self.tail.get() = next };
        let _ = unsafe { Box::from_raw(tail) };
        let ret = unsafe { (*next).value.take() };
        assert!(ret.is_some());
        self.len.fetch_sub(1, Ordering::Acquire);
        ret
    }
}

unsafe impl<T: Send> Send for SharedState<T> {}
unsafe impl<T: Send> Sync for SharedState<T> {}

impl<T> Drop for SharedState<T> {
    fn drop(&mut self) {
        let mut p = unsafe { *self.tail.get() };
        while !p.is_null() {
            let next = unsafe { (*p).next.load(Ordering::Acquire) };
            let _ = unsafe { Box::from_raw(p) };
            p = next;
        }
    }
}
