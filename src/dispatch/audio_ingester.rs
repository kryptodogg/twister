use crate::dispatch::signal_ingester::{SignalIngester, SignalMetadata, SampleFormat};
use crate::ml::field_particle::FieldParticle;
use std::collections::VecDeque;
use std::sync::Mutex;

pub struct AudioIngester {
    ring_buffer: Mutex<VecDeque<f32>>,
    accumulation_threshold: usize,
}

impl AudioIngester {
    pub fn new() -> Self {
        Self {
            ring_buffer: Mutex::new(VecDeque::with_capacity(8192)),
            accumulation_threshold: 4096,
        }
    }

    pub fn accumulate(&self, raw_signal: &[u8], metadata: &SignalMetadata) -> Option<Vec<f32>> {
        let sample_count = match metadata.sample_format {
            SampleFormat::I16 => raw_signal.len() / 2,
            SampleFormat::F32 => raw_signal.len() / 4,
            _ => 0,
        };
        if sample_count == 0 { return None; }
        let mut buf = self.ring_buffer.lock().ok()?;
        for i in 0..sample_count {
            let sample_f32 = match metadata.sample_format {
                SampleFormat::I16 => {
                    let mut bytes = [0u8; 2];
                    bytes.copy_from_slice(&raw_signal[i * 2..(i + 1) * 2]);
                    i16::from_le_bytes(bytes) as f32 / 32768.0
                }
                SampleFormat::F32 => {
                    let mut bytes = [0u8; 4];
                    bytes.copy_from_slice(&raw_signal[i * 4..(i + 1) * 4]);
                    f32::from_le_bytes(bytes)
                }
                _ => 0.0,
            };
            buf.push_back(sample_f32);
        }
        if buf.len() >= self.accumulation_threshold {
            let mut result = Vec::with_capacity(self.accumulation_threshold);
            for _ in 0..self.accumulation_threshold {
                if let Some(s) = buf.pop_front() {
                    result.push(s);
                }
            }
            Some(result)
        } else {
            None
        }
    }
}

impl SignalIngester for AudioIngester {
    fn ingest(
        &self,
        raw_signal: &[u8],
        timestamp_us: u64,
        metadata: &SignalMetadata,
    ) -> Vec<FieldParticle> {
        // UNBIASED INGESTER (BSS): No "Speech Enhancement" or filters.
        // Capturing raw physical intensity (including ultrasonic ripples).
        let mut particles = Vec::new();
        let sample_count = match metadata.sample_format {
            SampleFormat::I16 => raw_signal.len() / 2,
            SampleFormat::F32 => raw_signal.len() / 4,
            _ => 0,
        };
        if sample_count == 0 { return particles; }
        particles.reserve(sample_count);
        for i in 0..sample_count {
            let intensity = match metadata.sample_format {
                SampleFormat::I16 => {
                    let mut bytes = [0u8; 2];
                    bytes.copy_from_slice(&raw_signal[i * 2..(i + 1) * 2]);
                    (i16::from_le_bytes(bytes) as f32 / 32768.0).abs()
                }
                SampleFormat::F32 => {
                    let mut bytes = [0u8; 4];
                    bytes.copy_from_slice(&raw_signal[i * 4..(i + 1) * 4]);
                    f32::from_le_bytes(bytes).abs()
                }
                _ => 0.0,
            };
            particles.push(FieldParticle {
                position: [i as f32, 0.0, 0.0],
                intensity,
                color: [0.1, 0.4, 1.0, 1.0], // Resonant blue for audio
                source_id: 0, // Mic
                confidence: [0.0, 0.0, 1.0, 0.0], // High CV Inference/Intensity
                timestamp_us: timestamp_us + (i as u64),
                freq_hz: metadata.carrier_freq_hz.unwrap_or(440.0),
                phase_coherence: 1.0,
                doppler_shift: 0.0,
                bandwidth_hz: 0.0,
                anomaly_score: 0.0,
                material_id: 0,
                motif_hint: 255,
                scattering_cross_section: 0.0,
                permittivity_real: 0.0,
                permittivity_imag: 0.0,
                reserved_for_h2_null_phase: 0.0,
                reserved_for_ha_haptic_freq: 0.0,
                reserved_for_grb_water_saturation: 0.0,
                reserved_for_j1_proprioception: 0.0,
                reserved_for_forensic_hash_lo: 0,
            });
        }
        particles
    }
}
