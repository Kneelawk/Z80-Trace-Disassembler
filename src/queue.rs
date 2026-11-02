use std::ops::Index;

pub struct FixedQueue<T, const N: usize> {
    head: usize,
    data: [T; N],
}

impl<T, const N: usize> FixedQueue<T, N> {
    pub fn new_filled(fill: T) -> Self
    where
        T: Copy,
    {
        Self {
            head: 0,
            data: [fill; N],
        }
    }

    pub fn get(&self, index: usize) -> &T {
        let idx = (self.head + index) % N;
        &self.data[idx]
    }

    pub fn push(&mut self, data: T) {
        if self.head < 1 {
            self.head = N - 1;
        } else {
            self.head -= 1;
        }
        self.data[self.head] = data;
    }
}

impl<T, const N: usize> Index<usize> for FixedQueue<T, N> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        self.get(index)
    }
}
