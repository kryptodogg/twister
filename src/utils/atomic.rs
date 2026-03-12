// src/utils/atomic.rs — Lock-free atomic f32 for real-time DSP hot paths
// V3 Pattern: AtomicF32 wraps AtomicU32 with f32::to_bits/from_bits transmutation
// Used for: ring buffer state, anomaly scores, UI telemetry — anywhere f32 needs atomic access

use std::sync::atomic::{AtomicU32, Ordering};

/// Lock-free atomic f32 for real-time DSP hot paths.
/// 
/// # V3 Architecture Note
/// This is the ONLY AtomicF32 in the codebase. Do not create alternatives.
/// All f32 values crossing thread boundaries use this wrapper.
/// 
/// # Example
/// ```rust
/// use crate::utils::atomic::AtomicF32;
/// use std::sync::atomic::Ordering;
/// 
/// let score = AtomicF32::new(0.0);
/// score.store(0.85, Ordering::Relaxed);
/// let current = score.load(Ordering::Relaxed);
/// assert_eq!(current, 0.85);
/// ```
pub struct AtomicF32(AtomicU32);

impl AtomicF32 {
    /// Create a new AtomicF32 with the given initial value.
    #[inline]
    pub fn new(v: f32) -> Self {
        Self(AtomicU32::new(v.to_bits()))
    }

    /// Load the current value with the specified memory ordering.
    #[inline]
    pub fn load(&self, ord: Ordering) -> f32 {
        f32::from_bits(self.0.load(ord))
    }

    /// Store a new value with the specified memory ordering.
    #[inline]
    pub fn store(&self, v: f32, ord: Ordering) {
        self.0.store(v.to_bits(), ord);
    }

    /// Atomic swap: store new value, return old value.
    #[inline]
    pub fn swap(&self, v: f32, ord: Ordering) -> f32 {
        f32::from_bits(self.0.swap(v.to_bits(), ord))
    }

    /// Atomic compare-and-swap.
    /// Returns (old_value, success) where old_value is the value before the operation.
    #[inline]
    pub fn compare_exchange(
        &self,
        current: f32,
        new: f32,
        success: Ordering,
        failure: Ordering,
    ) -> Result<f32, f32> {
        self.0
            .compare_exchange(current.to_bits(), new.to_bits(), success, failure)
            .map(f32::from_bits)
            .map_err(f32::from_bits)
    }

    /// Atomic fetch-and-add (interprets bits as f32, not integer).
    /// Note: This is rarely useful for f32 — use only for specific DSP patterns.
    #[inline]
    pub fn fetch_add(&self, v: f32, ord: Ordering) -> f32 {
        self.0
            .fetch_update(ord, ord, |bits| {
                let current = f32::from_bits(bits);
                Some((current + v).to_bits())
            })
            .map(f32::from_bits)
            .unwrap()
    }
}

impl Default for AtomicF32 {
    fn default() -> Self {
        Self::new(0.0)
    }
}

impl std::fmt::Debug for AtomicF32 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AtomicF32({})", self.load(Ordering::Relaxed))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_atomic_f32_basic() {
        let v = AtomicF32::new(1.5);
        assert_eq!(v.load(Ordering::Relaxed), 1.5);
        v.store(2.5, Ordering::Relaxed);
        assert_eq!(v.load(Ordering::Relaxed), 2.5);
    }

    #[test]
    fn test_atomic_f32_concurrent() {
        let v = AtomicF32::new(0.0);
        let mut handles = vec![];

        for i in 0..4 {
            let v = &v;
            handles.push(thread::spawn(move || {
                for _ in 0..100 {
                    let old = v.fetch_add(0.1, Ordering::Relaxed);
                    assert!(old >= 0.0);
                }
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        // Should be approximately 4.0 (4 threads × 100 × 0.1)
        let final_val = v.load(Ordering::Relaxed);
        assert!((final_val - 4.0).abs() < 0.01);
    }
}
