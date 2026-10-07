use crate::terminal::Key;
use std::collections::VecDeque;

const CAPACITY: usize = 16;

pub struct Keyboard {
    buffer: VecDeque<Key>,
}

impl Keyboard {
    pub fn new() -> Self {
        Keyboard { buffer: VecDeque::with_capacity(CAPACITY) }
    }

    pub fn push(&mut self, key: Key) {
        if self.buffer.len() < CAPACITY {
            self.buffer.push_back(key);
        }
    }

    pub fn pop(&mut self) -> Option<Key> {
        self.buffer.pop_front()
    }

    pub fn peek(&self) -> Option<Key> {
        self.buffer.front().copied()
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
    }
}
