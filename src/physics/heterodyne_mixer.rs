use num_complex::Complex;

pub struct HeterodyneMixer {
    pub primary_freq: f32,           // Attack carrier (e.g., 2.4 GHz)
    pub modulation_freq: f32,        // Audio modulation (e.g., 4 kHz)
    pub heterodyne_freqs: Vec<f32>,  // Sideband frequencies
}

impl HeterodyneMixer {
    pub fn new(primary_hz: f32, modulation_hz: f32) -> Self {
        let lower_sideband = primary_hz - modulation_hz;
        let upper_sideband = primary_hz + modulation_hz;
        Self {
            primary_freq: primary_hz,
            modulation_freq: modulation_hz,
            heterodyne_freqs: vec![lower_sideband, primary_hz, upper_sideband],
        }
    }

    /// Mix two signals: a(t) * cos(ω₁t) × cos(ω₂t) = 0.5*cos((ω₁-ω₂)t) + 0.5*cos((ω₁+ω₂)t)
    pub fn mix_signals(
        &self,
        rf_field: Complex<f32>,
        audio_modulation: f32,
    ) -> Vec<Complex<f32>> {
        // RF × Audio modulation produces sidebands
        // Primary component (RF)
        // Lower sideband (f_primary - f_audio)
        // Upper sideband (f_primary + f_audio)

        vec![
            rf_field * 0.5 * Complex::new(audio_modulation.cos(), -audio_modulation.sin()),  // lower_sideband
            rf_field,                                                                        // primary
            rf_field * 0.5 * Complex::new(audio_modulation.cos(), audio_modulation.sin()),   // upper_sideband
        ]
    }

    /// Energy in sideband relative to carrier
    pub fn sideband_efficiency(&self) -> f32 {
        // Modulation index: m = f_audio / f_carrier (for AM)
        let m = self.modulation_freq / self.primary_freq;
        // Sideband power: P_sidebands = (m²/4) * P_carrier
        (m * m / 4.0).min(1.0)
    }
}
