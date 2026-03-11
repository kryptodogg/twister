use std::sync::atomic::{AtomicU64, Ordering};

pub struct QpcTimer {
    epoch_qpc: u64,
    frequency: u64,
}

impl QpcTimer {
    #[cfg(windows)]
    pub fn new() -> Self {
        use windows_sys::Win32::System::Performance::{QueryPerformanceCounter, QueryPerformanceFrequency};
        let mut freq = 0i64;
        let mut start = 0i64;
        unsafe {
            QueryPerformanceFrequency(&mut freq);
            QueryPerformanceCounter(&mut start);
        }
        Self {
            epoch_qpc: start as u64,
            frequency: freq as u64,
        }
    }

    #[cfg(not(windows))]
    pub fn new() -> Self {
        // Fallback for non-windows (not for production forensic use)
        Self {
            epoch_qpc: 0,
            frequency: 1_000_000,
        }
    }

    pub fn now_us(&self) -> u64 {
        #[cfg(windows)]
        {
            use windows_sys::Win32::System::Performance::QueryPerformanceCounter;
            let mut now = 0i64;
            unsafe { QueryPerformanceCounter(&mut now); }
            let elapsed = (now as u64).saturating_sub(self.epoch_qpc);
            (elapsed * 1_000_000) / self.frequency
        }
        #[cfg(not(windows))]
        {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_micros() as u64
        }
    }
}
