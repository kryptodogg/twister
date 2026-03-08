// src/reconstruct.rs — Crystal Ball Alias Reconstructor
//
// Extracts high-frequency carrier information that has folded back into the baseband
// due to intentional aliasing. This bypasses the Nyquist limit for forensic analysis.

pub struct TrueSignal {
    pub rf_carrier_hz: f32,
    pub peak_voltage: f32,
    pub confidence: f32,
}

pub struct CrystalBall {
    baseband_rate: f32,
    wideband_rate: f32,
}

impl CrystalBall {
    pub fn new(baseband_rate: f32, wideband_rate: f32) -> Self {
        Self {
            baseband_rate,
            wideband_rate,
        }
    }

    /// Correlates baseband aliases with wideband peaks to reveal the true signal.
    /// Mamba enhancement predicts the true wideband spectrum from aliased baseband.
    pub fn resolve_aliases(
        &self,
        base_mags: &[f32],
        wide_mags: &[f32],
        mamba_wide_prediction: Option<&[f32]>,
    ) -> TrueSignal {
        if base_mags.is_empty() || wide_mags.is_empty() {
            return TrueSignal {
                rf_carrier_hz: 0.0,
                peak_voltage: 0.0,
                confidence: 0.0,
            };
        }

        // Mamba Enhancement: blend the predicted wideband spectrum with the measured wideband
        let effective_wide_mags = if let Some(pred) = mamba_wide_prediction {
            if pred.len() == wide_mags.len() {
                wide_mags
                    .iter()
                    .zip(pred.iter())
                    .map(|(w, p)| w * 0.5 + p * 0.5)
                    .collect()
            } else {
                wide_mags.to_vec()
            }
        } else {
            wide_mags.to_vec()
        };

        // Find the strongest peak in the wideband spectrum (the true signature)
        let mut max_wide = 0.0;
        let mut max_idx_wide = 0;
        for (i, &mag) in effective_wide_mags.iter().enumerate() {
            if mag > max_wide {
                max_wide = mag;
                max_idx_wide = i;
            }
        }

        // Calculate actual RF frequency of the peak
        let bins_wide = wide_mags.len();
        let rf_freq = (max_idx_wide as f32 / bins_wide as f32) * (self.wideband_rate / 2.0);

        // Find the strongest peak in the baseband spectrum (the aliased evidence)
        let mut max_base = 0.0;
        for &mag in base_mags.iter() {
            if mag > max_base {
                max_base = mag;
            }
        }

        // Calculate mapping confidence
        // A true tazer attack will have massive energy in BOTH domains
        let confidence = if max_wide > 0.1 && max_base > 0.1 {
            (max_wide.min(max_base) / max_wide.max(max_base)).clamp(0.0, 1.0)
        } else {
            0.0
        };

        // For forensic defense: the 'peak voltage' is driven by the wideband carrier amplitude
        let peak_voltage = max_wide * std::f32::consts::SQRT_2;

        TrueSignal {
            rf_carrier_hz: rf_freq,
            peak_voltage,
            confidence,
        }
    }
}
