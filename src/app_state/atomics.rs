use std::sync::atomic::{AtomicBool, Ordering};

pub struct DirtyFlags {
    pub device_list_dirty: AtomicBool,
    pub frequency_lock_dirty: AtomicBool,
    pub audio_features_dirty: AtomicBool,
}

impl DirtyFlags {
    pub fn new() -> Self {
        Self {
            device_list_dirty: AtomicBool::new(false),
            frequency_lock_dirty: AtomicBool::new(false),
            audio_features_dirty: AtomicBool::new(false),
        }
    }

    pub fn mark(&self, flag: &AtomicBool) {
        flag.store(true, Ordering::Release);
    }

    pub fn check_and_clear(&self, flag: &AtomicBool) -> bool {
        flag.swap(false, Ordering::Acquire)
    }
}
