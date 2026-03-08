// src/state/atomic_f32.rs

use std::sync::atomic::{AtomicU32, Ordering};

pub struct AtomicF32 {
    inner: AtomicU32,
}

impl AtomicF32 {
    #[inline]
    pub const fn new(val: f32) -> Self {
        Self {
            inner: AtomicU32::new(val.to_bits()),
        }
    }

    #[inline]
    pub fn load(&self, order: Ordering) -> f32 {
        f32::from_bits(self.inner.load(order))
    }

    #[inline]
    pub fn store(&self, val: f32, order: Ordering) {
        self.inner.store(val.to_bits(), order);
    }
}
