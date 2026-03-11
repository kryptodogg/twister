use std::sync::atomic::{AtomicU32, Ordering};

pub struct MambaMetricsAtomics {
    pub drive: AtomicU32,
    pub fold: AtomicU32,
    pub asym: AtomicU32,
    pub anomaly: AtomicU32,
}

impl MambaMetricsAtomics {
    pub fn new() -> Self {
        Self {
            drive: AtomicU32::new(0f32.to_bits()),
            fold: AtomicU32::new(0f32.to_bits()),
            asym: AtomicU32::new(0f32.to_bits()),
            anomaly: AtomicU32::new(0f32.to_bits()),
        }
    }

    pub fn set_drive(&self, val: f32) { self.drive.store(val.to_bits(), Ordering::Relaxed); }
    pub fn get_drive(&self) -> f32 { f32::from_bits(self.drive.load(Ordering::Relaxed)) }

    pub fn set_fold(&self, val: f32) { self.fold.store(val.to_bits(), Ordering::Relaxed); }
    pub fn get_fold(&self) -> f32 { f32::from_bits(self.fold.load(Ordering::Relaxed)) }

    pub fn set_asym(&self, val: f32) { self.asym.store(val.to_bits(), Ordering::Relaxed); }
    pub fn get_asym(&self) -> f32 { f32::from_bits(self.asym.load(Ordering::Relaxed)) }

    pub fn set_anomaly(&self, val: f32) { self.anomaly.store(val.to_bits(), Ordering::Relaxed); }
    pub fn get_anomaly(&self) -> f32 { f32::from_bits(self.anomaly.load(Ordering::Relaxed)) }
}
