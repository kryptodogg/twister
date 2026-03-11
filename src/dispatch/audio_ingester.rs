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

    pub fn clear(&self) {
        if let Ok(mut buf) = self.ring_buffer.lock() {
            buf.clear();
        }
    }

    /// Accumulate raw bytes and return a 4096-sample buffer if threshold met
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
                    let val = i16::from_le_bytes(bytes);
                    val as f32 / 32768.0
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
        let mut particles = Vec::new();

        let sample_count = match metadata.sample_format {
            SampleFormat::I16 => raw_signal.len() / 2,
            SampleFormat::F32 => raw_signal.len() / 4,
            _ => 0,
        };

        if sample_count == 0 { return particles; }

        particles.reserve(sample_count);

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

            particles.push(FieldParticle {
                timestamp_us: timestamp_us + (i as u64),
                freq_hz: metadata.carrier_freq_hz.unwrap_or(0.0),
                energy: sample_f32.abs(),
                phase_coherence: 1.0,
                position_xyz: [i as f32, 0.0, 0.0],
                material_id: 0,
                source: 0,
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

        particles
    }
}
