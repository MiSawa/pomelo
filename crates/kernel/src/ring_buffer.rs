use core::mem::MaybeUninit;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct FullError<T> {
    element: T,
}
#[allow(dead_code)]
impl<T> FullError<T> {
    pub const fn new(element: T) -> Self {
        Self { element }
    }

    pub fn element(&self) -> &T {
        &self.element
    }

    pub fn take(self) -> T {
        self.element
    }
}

pub struct ArrayRingBuffer<T, const N: usize> {
    buffer: [MaybeUninit<T>; N],
    l: usize,
    r: usize,
}

impl<T, const N: usize> Default for ArrayRingBuffer<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
impl<T, const N: usize> ArrayRingBuffer<T, N> {
    pub const fn new() -> Self {
        Self {
            buffer: MaybeUninit::uninit_array(),
            l: 0,
            r: 0,
        }
    }

    pub const fn len(&self) -> usize {
        self.r - self.l
    }

    pub const fn is_empty(&self) -> bool {
        self.l == self.r
    }

    pub const fn capacity(&self) -> usize {
        N
    }

    pub const fn is_full(&self) -> bool {
        self.capacity() == self.len()
    }

    pub const fn remaining_capacity(&self) -> usize {
        self.capacity() - self.len()
    }

    pub fn clear(&mut self) {
        self.l = 0;
        self.r = 0;
    }

    pub fn back(&self) -> Option<&T> {
        if self.is_empty() {
            None
        } else {
            let i = self.r - 1;
            let i = if i >= N { i - N } else { i };
            Some(unsafe { self.buffer[i].assume_init_ref() })
        }
    }

    pub fn back_mut(&mut self) -> Option<&mut T> {
        if self.is_empty() {
            None
        } else {
            let i = self.r - 1;
            let i = if i >= N { i - N } else { i };
            Some(unsafe { self.buffer[i].assume_init_mut() })
        }
    }

    pub fn front(&self) -> Option<&T> {
        if self.is_empty() {
            None
        } else {
            Some(unsafe { self.buffer[self.l].assume_init_ref() })
        }
    }

    pub fn front_mut(&mut self) -> Option<&mut T> {
        if self.is_empty() {
            None
        } else {
            Some(unsafe { self.buffer[self.l].assume_init_mut() })
        }
    }

    /// ***Panics*** if the buffer is already full.
    pub fn push_back(&mut self, element: T) {
        self.try_push_back(element)
            .map_err(|_| ())
            .expect("This buffer is already full")
    }

    pub fn try_push_back(&mut self, element: T) -> Result<(), FullError<T>> {
        let r = if self.r >= N {
            let r = self.r - N;
            if self.l == r {
                return Err(FullError::new(element));
            }
            r
        } else {
            self.r
        };
        self.buffer[r].write(element);
        self.r += 1;
        Ok(())
    }

    /// ***Panics*** if the buffer is already full.
    pub fn push_front(&mut self, element: T) {
        self.try_push_front(element)
            .map_err(|_| ())
            .expect("This buffer is already full")
    }

    pub fn try_push_front(&mut self, element: T) -> Result<(), FullError<T>> {
        if self.is_full() {
            return Err(FullError::new(element));
        }
        if self.l == 0 {
            self.l = N - 1;
            self.r += N;
        } else {
            self.l -= 1;
        }
        self.buffer[self.l].write(element);
        Ok(())
    }

    pub fn pop_front(&mut self) -> Option<T> {
        if self.is_empty() {
            return None;
        }
        let ret = core::mem::replace(&mut self.buffer[self.l], MaybeUninit::uninit());
        // SAFETY: This slot should be occupied.
        let ret = unsafe { ret.assume_init() };
        self.l += 1;
        if self.l == N {
            self.l = 0;
            self.r -= N;
        }
        Some(ret)
    }

    pub fn pop_back(&mut self) -> Option<T> {
        if self.is_empty() {
            return None;
        }
        self.r -= 1;
        let r = if self.r < N { self.r } else { self.r - N };
        let ret = core::mem::replace(&mut self.buffer[r], MaybeUninit::uninit());
        // SAFETY: This slot should be occupied.
        let ret = unsafe { ret.assume_init() };
        Some(ret)
    }

    pub fn iter(&self) -> impl Iterator<Item = &'_ T> {
        (self.l..self.r).map(|i| {
            if i < N {
                unsafe { self.buffer[i].assume_init_ref() }
            } else {
                unsafe { self.buffer[i - N].assume_init_ref() }
            }
        })
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        fn unwrapper<T>(v: &mut MaybeUninit<T>) -> &mut T {
            unsafe { v.assume_init_mut() }
        }
        if self.r > N {
            let (ab, c) = self.buffer.split_at_mut(self.l);
            let (a, _b) = ab.split_at_mut(self.r - N);
            a.iter_mut().chain(c).map(unwrapper)
        } else {
            let (ab, _c) = self.buffer.split_at_mut(self.r);
            let (a, b) = ab.split_at_mut(self.l);
            let (empty, _) = a.split_at_mut(0);
            b.iter_mut().chain(empty).map(unwrapper)
        }
    }
}

impl<T, const N: usize> core::ops::Index<usize> for ArrayRingBuffer<T, N> {
    type Output = T;
    fn index(&self, index: usize) -> &Self::Output {
        assert!(index < self.len());
        let mut i = self.l + index;
        if i >= N {
            i -= N;
        }
        unsafe { self.buffer[i].assume_init_ref() }
    }
}

impl<T, const N: usize> core::ops::IndexMut<usize> for ArrayRingBuffer<T, N> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        assert!(index < self.len());
        let mut i = self.l + index;
        if i >= N {
            i -= N;
        }
        unsafe { self.buffer[i].assume_init_mut() }
    }
}
