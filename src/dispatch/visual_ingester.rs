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
        if raw_signal.is_empty() {
            return Vec::new();
        }

        // Zero-mock: Only process real video frames if provided.
        // Expecting raw RGB or Grayscale bytes.
        // For Track A, we provide a placeholder that converts real pixel intensities
        // into particles without synthesizing fake positions or noise.

        let mut particles = Vec::new();
        let pixel_count = raw_signal.len(); // Simple assumption: 1 byte per pixel for intensity

        // Sampling strategy to avoid particle explosion: take every 64th pixel
        for i in (0..pixel_count).step_by(64) {
            let intensity = raw_signal[i] as f32 / 255.0;
            if intensity > 0.1 { // Threshold to ignore dark background
                particles.push(FieldParticle {
                    timestamp_us,
                    freq_hz: 500e12, // Representative optical frequency (500 THz)
                    energy: intensity,
                    phase_coherence: 0.5,
                    position_xyz: [ (i % 640) as f32 / 640.0, (i / 640) as f32 / 480.0, 0.0 ],
                    material_id: 11, // Optical bucket
                    source: 2, // HostProcessed
                    _pad0: [0; 2],
                    doppler_shift: 0.0,
                    phase_velocity: 0.0,
                    scattering_cross_section: 0.0,
                    bandwidth_hz: 0.0,
                    anomaly_score: 0.0,
                    motif_hint: 255,
                    _pad1: [0; 3],
                    embedding: [0.0; 16],
                });
            }
        }

        particles
    }
}
