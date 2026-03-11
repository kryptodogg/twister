pub mod atomics;
pub use atomics::MambaMetricsAtomics;

use std::sync::atomic::{AtomicBool, Ordering};

pub struct HologramStatus {
    pub audio_live: AtomicBool,
    pub rf_live: AtomicBool,
    pub optical_live: AtomicBool,
}

impl HologramStatus {
    pub fn new() -> Self {
        Self {
            audio_live: AtomicBool::new(false),
            rf_live: AtomicBool::new(false),
            optical_live: AtomicBool::new(false),
        }
    }
}
