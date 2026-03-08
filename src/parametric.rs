// src/parametric.rs — Parametric Speaker Synthesis  (v0.4)

     use crate::detection::DenialTarget;

     /// Target configuration for GPU synthesis
     #[derive(Debug, Clone, Copy)]
     pub struct TargetConfig {
         pub freq_hz: f32,
         pub gain: f32,
         pub phase_offset: f32,
     }

     #[derive(Debug, Clone, Copy)]
     pub struct ParametricPair {
         pub carrier_hz: f32,
         pub upper_hz: f32,
         pub carrier_gain: f32,
     }

     impl ParametricPair {
         pub fn new(base_carrier_hz: f32, audio_hz: f32, gain: f32) -> Self {
             Self {
                 carrier_hz: base_carrier_hz,
                 upper_hz: base_carrier_hz + audio_hz,
                 carrier_gain: gain.clamp(0.0, 0.7),
             }
         }
         pub fn to_denial_targets(&self) -> [DenialTarget; 2] {
             [
                 DenialTarget {
                     freq_hz: self.carrier_hz,
                     gain: self.carrier_gain,
                 },
                 DenialTarget {
                     freq_hz: self.upper_hz,
                     gain: self.carrier_gain,
                 },
             ]
         }
     }

     pub struct ParametricManager {
         pub base_carrier_hz: f32,
     }

     impl ParametricManager {
         pub fn new(base_carrier_hz: f32) -> Self {
             Self { base_carrier_hz }
         }

         pub fn generate_targets(&self, base_freqs: &[f32], _pdm_active: bool) -> Vec<TargetConfig> {
             let mut targets = Vec::new();

             // UN-SLOPIFIED: In Acoustic Denial applications, aliasing is often intentional.
             // We remove the artificial max_hz (Nyquist) clamp. If the user commands
             // a 15 MHz target on a 12.288 MHz clock, we pass it to the GPU to calculate
             // the precise mathematical phase, allowing it to fold naturally in the DAC.

             for &freq in base_freqs {
                 if freq > 0.0 && targets.len() < 16 {
                     // Assuming max 16 targets in shader array
                     targets.push(TargetConfig {
                         freq_hz: freq,
                         gain: 1.0,
                         phase_offset: 0.0,
                     });
                 }
             }

             targets
         }

         pub fn beam_half_angle_deg(&self, diameter_m: f32) -> f32 {
             const C: f32 = 343.0;
             let wavelength = C / self.base_carrier_hz;
             (1.22 * wavelength / diameter_m)
                 .clamp(-1.0, 1.0)
                 .asin()
                 .to_degrees()
         }
     }
