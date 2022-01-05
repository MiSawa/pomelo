use core::{
    cell::UnsafeCell,
    sync::atomic::{AtomicPtr, Ordering},
};

use alloc::{boxed::Box, sync::Arc};

pub struct MPSCConsumer<T> {
    state: Arc<SharedState<T>>,
}
impl<T> MPSCConsumer<T> {
    pub fn new() -> Self {
        Self {
            state: Arc::new(SharedState::new()),
        }
    }
    pub fn pop(&mut self) -> Option<T> {
        self.state.pop()
    }
    pub fn producer(&self) -> MPSCProducer<T> {
        MPSCProducer {
            state: self.state.clone(),
        }
    }
}

#[derive(Clone)]
pub struct MPSCProducer<T> {
    state: Arc<SharedState<T>>,
}
impl<T> MPSCProducer<T> {
    pub fn push(&self, value: T) {
        self.state.push(value);
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
    fn to_ptr(self) -> *mut Self {
        Box::into_raw(Box::new(self))
    }
}
struct SharedState<T> {
    head: AtomicPtr<Node<T>>,
    tail: UnsafeCell<*mut Node<T>>,
}

impl<T> SharedState<T> {
    fn new() -> Self {
        let dummy = Node::dummy().to_ptr();
        Self {
            head: AtomicPtr::new(dummy),
            tail: UnsafeCell::new(dummy),
        }
    }

    fn push(&self, value: T) {
        let node = Node::new(value).to_ptr();
        let previous_head = self.head.swap(node, Ordering::AcqRel);
        unsafe { (*previous_head).next.store(node, Ordering::Release) };
    }

    fn pop(&self) -> Option<T> {
        let tail = unsafe { *self.tail.get() };
        let next = unsafe { (*tail).next.load(Ordering::Acquire) };
        if next.is_null() {
            return None;
        }
        unsafe { *self.tail.get() = next };
        let _ = unsafe { Box::from_raw(tail) };
        let ret = unsafe { (*next).value.take() };
        assert!(ret.is_some());
        ret
    }
}

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
