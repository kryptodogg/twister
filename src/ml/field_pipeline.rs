use crate::dispatch::{AudioIngester, RFIngester, VisualIngester, SignalIngester, SignalMetadata, SampleFormat, SignalType};
use crate::ml::unified_field_mamba::{UnifiedFieldMamba, HitListAccumulator};
use burn::backend::Wgpu;

/// Audio input source variant
/// Supports local microphone, iPhone web app, and other future sources
#[derive(Clone, Debug)]
pub enum AudioSource {
    /// Local microphone (e.g., Logitech C925e USB)
    LocalMicrophone { sample_rate: u32 },
    /// iPhone with web app (WebRTC or HTTP streaming)
    IPhoneWebApp { device_id: String, sample_rate: u32 },
    /// Future: networked device, streaming service, etc.
    Custom { label: String },
}

impl Default for AudioSource {
    fn default() -> Self {
        // Default to local microphone at 48 kHz
        Self::LocalMicrophone { sample_rate: 48_000 }
    }
}

/// Output scalars derived from FieldParticle stream
#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct MambaProjections {
    pub drive: f32,
    pub fold: f32,
    pub asym: f32,
}

/// Simple real-time field pipeline combining ingesters, accumulator, and Mamba
/// Supports variable audio sources (local mic, iPhone web app, etc.)
pub struct FieldPipeline {
    audio: AudioIngester,
    audio_source: AudioSource,
    rf: RFIngester,
    visual: VisualIngester,
    accumulator: HitListAccumulator,
    pub mamba: UnifiedFieldMamba<Wgpu>,
    pub device: burn::tensor::Device<Wgpu>,
}

impl FieldPipeline {
    pub fn new() -> Self {
        Self::with_audio_source(AudioSource::default())
    }

    /// Create pipeline with a specific audio source
    pub fn with_audio_source(audio_source: AudioSource) -> Self {
        let device = burn_wgpu::WgpuDevice::default();
        let mamba = UnifiedFieldMamba::new(&device);
        Self {
            audio: AudioIngester::new(),
            audio_source,
            rf: RFIngester::new(),
            visual: VisualIngester::new(640, 480), // Default C925e webcam resolution
            accumulator: HitListAccumulator::new(),
            mamba,
            device,
        }
    }

    /// Get the current audio source configuration
    pub fn audio_source(&self) -> &AudioSource {
        &self.audio_source
    }

    /// Update the audio source (e.g., switch from local mic to iPhone web app)
    pub fn set_audio_source(&mut self, source: AudioSource) {
        self.audio_source = source;
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
            SignalType::Video => self.visual.ingest(bytes, timestamp_us, metadata),
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

    /// Ingest a video frame from the visual microphone (C925e webcam)
    /// Returns projections when the accumulator flushes
    pub fn ingest_video_frame(
        &mut self,
        frame_rgb: &[u8],
        timestamp_us: u64,
    ) -> Option<MambaProjections> {
        let metadata = SignalMetadata {
            signal_type: SignalType::Video,
            sample_rate_hz: 30, // ~30 fps typical
            carrier_freq_hz: None,
            num_channels: 3,
            sample_format: SampleFormat::F32, // RGB24 encoded
        };
        self.ingest_bytes(frame_rgb, timestamp_us, &metadata)
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
