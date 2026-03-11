use crate::dispatch::signal_ingester::{SignalIngester, SignalMetadata};
use crate::ml::field_particle::FieldParticle;

pub struct VisualIngester;

impl VisualIngester {
    pub fn new() -> Self {
        Self
    }
}

impl SignalIngester for VisualIngester {
    fn ingest(
        &self,
        raw_signal: &[u8],
        timestamp_us: u64,
        _metadata: &SignalMetadata,
    ) -> Vec<FieldParticle> {
        if raw_signal.is_empty() { return Vec::new(); }

        // UNBIASED INGESTER (BSS) + Pose Integration
        let mut particles = Vec::new();
        let pixel_count = raw_signal.len();

        // 1. Raw Intensity Mapping (BSS)
        for i in (0..pixel_count).step_by(128) {
            let intensity = raw_signal[i] as f32 / 255.0;
            if intensity > 0.15 {
                particles.push(FieldParticle {
                    position: [ (i % 640) as f32 / 640.0, (i / 640) as f32 / 480.0, 0.0 ],
                    intensity,
                    color: [intensity, intensity * 0.8, 0.2, 1.0],
                    source_id: 3, // CMOS
                    confidence: [1.0, 0.5, 0.0, 0.0],
                    timestamp_us,
                    freq_hz: 500e12,
                    phase_coherence: 1.0,
                    doppler_shift: 0.0,
                    bandwidth_hz: 0.0,
                    anomaly_score: 0.0,
                    material_id: 11,
                    motif_hint: 255,
                    _padding: [0.0; 8],
                });
            }
        }

        // 2. Pose Data (Injected as high-confidence holographic points)
        // [In future track, this would call PoseEstimator::estimate]

        particles
    }
}
