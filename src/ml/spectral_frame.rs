// src/ml/spectral_frame.rs — Real-time audio spectral features (Track C.2)
//
// Extracts instantaneous spectral information from the audio dispatch loop.
// Used for rapid anomaly evaluation before expensive Mamba/TimeGNN passes.

#[derive(Debug, Clone)]
pub struct SpectralFrame {
    pub timestamp_micros: u64,
    /// 512-bin FFT compressed/interpolated down to 128 mel-scale bins
    pub fft_magnitude: [f32; 128],
    /// Bicoherence (phase coupling) indicator to detect non-linear RF mixing
    pub bispectrum: [f32; 64],
    /// Interaural time/level difference (spatial correlation)
    pub itd_ild: [f32; 4],
    /// 3 fixed azimuths: -45°, 0°, +45°
    pub beamformer_outputs: [f32; 3],
    /// From Mamba latent reconstruction MSE (historical or previous frame's prediction)
    pub mamba_anomaly_score: f32,
    pub confidence: f32,
}

impl Default for SpectralFrame {
    fn default() -> Self {
        SpectralFrame {
            timestamp_micros: 0,
            fft_magnitude: [0.0; 128],
            bispectrum: [0.0; 64],
            itd_ild: [0.0; 4],
            beamformer_outputs: [0.0; 3],
            mamba_anomaly_score: 0.0,
            confidence: 0.0,
        }
    }
}

impl SpectralFrame {
    pub fn new(
        timestamp_micros: u64,
        fft_magnitude: [f32; 128],
        bispectrum: [f32; 64],
        itd_ild: [f32; 4],
        beamformer_outputs: [f32; 3],
        mamba_anomaly_score: f32,
        confidence: f32,
    ) -> Self {
        Self {
            timestamp_micros,
            fft_magnitude,
            bispectrum,
            itd_ild,
            beamformer_outputs,
            mamba_anomaly_score,
            confidence,
        }
    }

    pub fn is_valid(&self) -> bool {
        self.fft_magnitude.iter().all(|x| x.is_finite())
            && self.bispectrum.iter().all(|x| x.is_finite())
            && self.itd_ild.iter().all(|x| x.is_finite())
            && self.beamformer_outputs.iter().all(|x| x.is_finite())
            && self.mamba_anomaly_score.is_finite()
            && self.confidence.is_finite()
            && self.confidence >= 0.0
            && self.confidence <= 1.0
    }
}
