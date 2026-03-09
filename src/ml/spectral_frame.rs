/// Audio-derived features produced by Track C.2 every ~100ms.
#[derive(Debug, Clone)]
pub struct SpectralFrame {
    pub timestamp_micros: u64,
    pub fft_magnitude: [f32; 128],      // Mel-scale binned FFT
    pub bispectrum: [f32; 64],          // Phase coupling
    pub itd_ild: [f32; 4],              // Interaural differences (TDOA)
    pub beamformer_outputs: [f32; 3],   // 3 fixed azimuths
    pub mamba_anomaly_score: f32,       // Primary gate threshold variable
    pub confidence: f32,                // Detection confidence (0-1)
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
        self.mamba_anomaly_score.is_finite() && self.confidence.is_finite() && self.confidence >= 0.0 && self.confidence <= 1.0
        && self.fft_magnitude.iter().all(|x| x.is_finite())
        && self.bispectrum.iter().all(|x| x.is_finite())
        && self.itd_ild.iter().all(|x| x.is_finite())
        && self.beamformer_outputs.iter().all(|x| x.is_finite())
    }
}
