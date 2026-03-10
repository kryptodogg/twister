use crate::dispatch::signal_ingester::{SignalIngester, SignalMetadata, SampleFormat, SignalType};
use crate::ml::field_particle::FieldParticle;

pub struct AudioIngester;

impl AudioIngester {
    pub fn new() -> Self {
        Self
    }
}

impl SignalIngester for AudioIngester {
    fn ingest(
        &self,
        raw_signal: &[u8],
        _timestamp_us: u64,
        metadata: &SignalMetadata,
    ) -> Vec<FieldParticle> {
        let mut particles = Vec::new();

        let sample_count = match metadata.sample_format {
            SampleFormat::I16 => raw_signal.len() / 2,
            SampleFormat::F32 => raw_signal.len() / 4,
            _ => 0, // Invalid for audio
        };

        if sample_count == 0 {
            return particles;
        }

        particles.reserve(sample_count);

        // Simple rolling RMS window for energy (approx)
        let mut sum_sq = 0.0;
        let window_size = 10;
        let mut window = std::collections::VecDeque::with_capacity(window_size);

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

            // Update RMS window
            let sq = sample_f32 * sample_f32;
            sum_sq += sq;
            window.push_back(sq);
            if window.len() > window_size {
                if let Some(old_sq) = window.pop_front() {
                    sum_sq -= old_sq;
                }
            }
            let energy = (sum_sq / window.len() as f32).sqrt();

            particles.push(FieldParticle {
                position: [i as f32, 0.0, 0.0], // Simple 1D spatial mapping for now
                phase_i: sample_f32,
                phase_q: 0.0, // Simplification for speed. A real Hilbert transform would map phase_q.
                energy,
                material_id: 0x0010, // Ultrasonic/audio latent cluster mapping
                _padding: [0; 3],
            });
        }

        particles
    }
}
