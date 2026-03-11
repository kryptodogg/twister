use crate::dispatch::signal_ingester::{SignalIngester, SignalMetadata, SampleFormat};
use crate::ml::field_particle::FieldParticle;

pub struct RFIngester;

impl RFIngester {
    pub fn new() -> Self {
        Self
    }
}

impl SignalIngester for RFIngester {
    fn ingest(
        &self,
        raw_signal: &[u8],
        timestamp_us: u64,
        metadata: &SignalMetadata,
    ) -> Vec<FieldParticle> {
        let mut particles = Vec::new();

        let sample_count = match metadata.sample_format {
            SampleFormat::IQ8 => raw_signal.len() / 2,
            SampleFormat::IQ16 => raw_signal.len() / 4,
            SampleFormat::IQ32F => raw_signal.len() / 8,
            _ => 0, // Invalid for RF
        };

        if sample_count == 0 {
            return particles;
        }

        particles.reserve(sample_count);

        let freq_hz = metadata.carrier_freq_hz.unwrap_or(0.0);

        for i in 0..sample_count {
            let (phase_i, phase_q) = match metadata.sample_format {
                SampleFormat::IQ8 => {
                    let i_val = (raw_signal[i * 2] as f32 - 127.5) / 128.0;
                    let q_val = (raw_signal[i * 2 + 1] as f32 - 127.5) / 128.0;
                    (i_val, q_val)
                }
                SampleFormat::IQ16 => {
                    let mut i_bytes = [0u8; 2];
                    i_bytes.copy_from_slice(&raw_signal[i * 4..(i * 4) + 2]);
                    let i_val = i16::from_le_bytes(i_bytes) as f32 / 32768.0;

                    let mut q_bytes = [0u8; 2];
                    q_bytes.copy_from_slice(&raw_signal[(i * 4) + 2..(i + 1) * 4]);
                    let q_val = i16::from_le_bytes(q_bytes) as f32 / 32768.0;

                    (i_val, q_val)
                }
                SampleFormat::IQ32F => {
                    let mut i_bytes = [0u8; 4];
                    i_bytes.copy_from_slice(&raw_signal[i * 8..(i * 8) + 4]);
                    let i_val = f32::from_le_bytes(i_bytes);

                    let mut q_bytes = [0u8; 4];
                    q_bytes.copy_from_slice(&raw_signal[(i * 8) + 4..(i + 1) * 8]);
                    let q_val = f32::from_le_bytes(q_bytes);

                    (i_val, q_val)
                }
                _ => (0.0, 0.0),
            };

            // Energy via sqrt(I^2 + Q^2)
            let energy = (phase_i * phase_i + phase_q * phase_q).sqrt();

            particles.push(FieldParticle {
                timestamp_us: timestamp_us + (i as u64),
                freq_hz,
                energy,
                phase_coherence: 1.0,
                position_xyz: [freq_hz as f32 / 1e9, i as f32, 0.0],
                material_id: 5, // Default RF bucket
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

        particles
    }
}
