use crate::dispatch::{AudioIngester, RFIngester, SignalIngester, SignalMetadata, SampleFormat, SignalType};
use crate::ml::field_particle::FieldParticle;
use crate::ml::unified_field_mamba::{UnifiedFieldMamba, HitListAccumulator};
use burn::backend::Wgpu;

/// Output scalars derived from FieldParticle stream
#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct MambaProjections {
    pub drive: f32,
    pub fold: f32,
    pub asym: f32,
}

/// Simple real-time field pipeline combining ingesters, accumulator, and Mamba
pub struct FieldPipeline {
    audio: AudioIngester,
    rf: RFIngester,
    accumulator: HitListAccumulator,
    pub mamba: UnifiedFieldMamba<Wgpu>,
    pub device: burn::tensor::Device<Wgpu>,
}

impl FieldPipeline {
    pub fn new() -> Self {
        let device = burn_wgpu::WgpuDevice::default();
        let mamba = UnifiedFieldMamba::new(&device);
        Self {
            audio: AudioIngester::new(),
            rf: RFIngester::new(),
            accumulator: HitListAccumulator::new(),
            mamba,
            device,
        }
    }

    /// Ingest raw bytes and return optional projections when a flush occurs
    pub fn ingest_bytes(
        &mut self,
        bytes: &[u8],
        timestamp_us: u64,
        metadata: &SignalMetadata,
    ) -> Option<MambaProjections> {
        let particles = match metadata.signal_type {
            SignalType::Audio => self.audio.ingest(bytes, timestamp_us, metadata),
            SignalType::RF => self.rf.ingest(bytes, timestamp_us, metadata),
            _ => Vec::new(),
        };

        // Convert particles to 9D vectors and add to accumulator
        let mut converted = Vec::new();
        for p in particles.iter() {
            converted.push([
                p.position[0],
                p.position[1],
                p.position[2],
                p.phase_i,
                p.phase_q,
                p.energy,
                0.0, // hardness placeholder
                0.0, // roughness placeholder
                0.0, // wetness placeholder
            ]);
        }

        if self.accumulator.extend(&converted) {
            let batch = self.accumulator.flush();
            // run through mamba
            let tensor = burn::tensor::Tensor::from_data(
                burn::tensor::TensorData::new(batch.into_iter().flatten().collect(),
                    [1, converted.len(), 9]),
                &self.device,
            );
            let (_out, _latent) = self.mamba.forward(tensor);
            // compute projections from raw particles for now
            Some(Self::compute_projections_from_particles(&converted))
        } else {
            None
        }
    }

    /// Projection heuristics reused from example
    fn compute_projections_from_particles(particles: &[[f32; 9]]) -> MambaProjections {
        if particles.is_empty() {
            return MambaProjections::default();
        }
        let drive = particles.iter().map(|p| p[3].abs() + p[4].abs()).sum::<f32>() / particles.len() as f32;
        let fold = particles.iter().map(|p| p[5]).sum::<f32>() / particles.len() as f32;
        let asym = particles.iter().map(|p| (p[3] - p[4]).abs()).sum::<f32>() / particles.len() as f32;
        MambaProjections { drive: drive.min(1.0), fold: fold.min(1.0), asym: asym.min(1.0) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn projections_vary_over_time() {
        let mut pipeline = FieldPipeline::new();
        let mut last = MambaProjections::default();
        // simulate feeding 1000 small audio chunks with varying energy
        for i in 0..1000 {
            let energy = (i as f32 / 1000.0).sin().abs();
            let sample = (energy * 32767.0) as i16;
            let bytes = sample.to_le_bytes();
            let metadata = SignalMetadata { signal_type: SignalType::Audio, sample_rate_hz: 48000, carrier_freq_hz: None, num_channels: 1, sample_format: SampleFormat::I16 };
            if let Some(proj) = pipeline.ingest_bytes(&bytes, i, &metadata) {
                // ensure at least one of the values differs from the previous batch
                assert!(proj != last, "projections did not change");
                last = proj;
            }
        }
    }
}
