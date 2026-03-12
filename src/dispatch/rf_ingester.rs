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
            _ => 0,
        };
        if sample_count == 0 { return particles; }
        particles.reserve(sample_count);
        let freq_hz = metadata.carrier_freq_hz.unwrap_or(0.0);
        for i in 0..sample_count {
            let (phase_i, phase_q) = match metadata.sample_format {
                SampleFormat::IQ32F => {
                    let mut i_bytes = [0u8; 4];
                    i_bytes.copy_from_slice(&raw_signal[i * 8..(i * 8) + 4]);
                    let mut q_bytes = [0u8; 4];
                    q_bytes.copy_from_slice(&raw_signal[(i * 8) + 4..(i + 1) * 8]);
                    (f32::from_le_bytes(i_bytes), f32::from_le_bytes(q_bytes))
                }
                _ => (0.0, 0.0), // Simplified for BSS Focus
            };
            let intensity = (phase_i * phase_i + phase_q * phase_q).sqrt();
            particles.push(FieldParticle {
                position: [freq_hz as f32 / 1e9, i as f32 / 1024.0, 0.0],
                intensity,
                color: [0.8, 0.2, 1.0, 1.0], // Resonant violet for RF
                source_id: 1, // SDR
                confidence: [0.0, 0.0, 0.0, 1.0], // [.., RF_Density]
                timestamp_us: timestamp_us + (i as u64),
                freq_hz,
                phase_coherence: 1.0,
                doppler_shift: 0.0,
                bandwidth_hz: 0.0,
                anomaly_score: 0.0,
                material_id: 5,
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
