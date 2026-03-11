use crate::ml::field_particle::FieldParticle;

/// Universal signal ingestion interface
/// Converts raw bytes (PCM, IQ, pixel data) -> FieldParticle stream
pub trait SignalIngester: Send + Sync {
    /// Ingest raw signal bytes and produce particles
    ///
    /// # Arguments
    /// * `raw_signal`: Raw sensor data (PCM audio, SDR IQ stream, video buffer, etc.)
    /// * `timestamp_us`: Microsecond timestamp for Hilbert ordering
    /// * `metadata`: SignalMetadata (sample rate, frequency, modulation, etc.)
    ///
    /// # Returns
    /// Vec<FieldParticle> - particles ready for Hilbert sorting and Mamba processing
    fn ingest(
        &self,
        raw_signal: &[u8],
        timestamp_us: u64,
        metadata: &SignalMetadata,
    ) -> Vec<FieldParticle>;
}

/// Metadata about the signal source
#[derive(Clone, Debug)]
pub struct SignalMetadata {
    pub signal_type: SignalType,      // Audio / RF / Video / Custom
    pub sample_rate_hz: u32,          // Sample rate or pixel clock
    pub carrier_freq_hz: Option<f64>, // RF carrier frequency, None for audio
    pub num_channels: u32,            // For multichannel audio or MIMO RF
    pub sample_format: SampleFormat,  // I16, F32, IQ8, etc.
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SignalType {
    Audio,           // PCM waveform
    RF,              // I/Q from SDR
    Video,           // RGB/YUV pixel data
    Radar,           // Radar returns
    Custom(u32),     // Extensible for future sensors
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SampleFormat {
    I16,             // 16-bit signed PCM
    F32,             // 32-bit float
    IQ8,             // 8-bit I + 8-bit Q (SDR)
    IQ16,            // 16-bit I + 16-bit Q (SDR)
}

// WGSL Equivalent (add as a comment for the GPU shader pipeline):
// struct FieldParticle {
//     position: vec3<f32>,
//     phase_i: f32,
//     phase_q: f32,
//     energy: f32,
//     material_id: u32,
//     _padding: vec3<u32>,
// };