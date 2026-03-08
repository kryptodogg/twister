import re

with open('src/ml/modular_features.rs', 'r') as f:
    text = f.read()

# 1. Add use_impulse_train to FeatureFlags
text = text.replace('pub use_harmonic_analysis: bool,  // Toggle harmonic relationships (32-D)', 'pub use_harmonic_analysis: bool,\n    pub use_impulse_train: bool,')
text = text.replace('use_harmonic_analysis: true,', 'use_harmonic_analysis: true,\n            use_impulse_train: true,')

# 2. Add impulse_train_data to Extractor
text = text.replace('harmonic_energy: Option<Vec<f32>>, // 32-D', 'harmonic_energy: Option<Vec<f32>>,\n    impulse_train: Option<Vec<f32>>,')

# 3. Compute it in from_payload
import_payload = """        // 6. Harmonic Energy (32-D)
        let harmonic_energy = payload.harmonic_data.as_ref().map(|hd| {
            let mut harm = vec![0.0f32; 32];
            for (i, &val) in hd.iter().take(32).enumerate() {
                harm[i] = (val * val).sqrt(); // RMS energy sim
            }
            harm
        });

        // 7. Impulse Train (580-D)
        let impulse_train = if payload.raw_audio.len() >= 512 {
            let mut stft = vec![0.0f32; 512];
            for (i, &val) in payload.raw_audio.iter().take(512).enumerate() {
                stft[i] = val.abs();
            }
            let detection = detect_impulses(&stft);
            let (spacing, jitter, conf) = measure_pulse_train_coherence(&payload.raw_audio, 192000);

            let mut features = ImpulseTrainFeatures {
                impulse_detection: detection,
                impulse_spacing: spacing,
                impulse_spacing_jitter: jitter,
                amplitude_envelope: [0.0; 64],
                impulse_phase_lock: 0.0,
                pulse_train_confidence: conf,
            };

            // basic envelope sim
            for i in 0..64 {
                features.amplitude_envelope[i] = payload.raw_audio.get(i * 8).cloned().unwrap_or(0.0).abs();
            }

            Some(features.to_vec())
        } else {
            None
        };
"""

text = re.sub(r'\s*// 6\. Harmonic Energy \(32-D\).*?harm\n        \}\);\n', import_payload, text, flags=re.DOTALL)

text = text.replace('harmonic_energy,\n        }', 'harmonic_energy,\n            impulse_train,\n        }')

# 4. Update extract() method to handle 941 dimensions
text = text.replace('let mut features = vec![0.0f32; 361];', 'let mut features = vec![0.0f32; 941];')
text = text.replace('let mut mask = vec![0.0f32; 361];', 'let mut mask = vec![0.0f32; 941];')

extract_impulse = """        // offset += 32; Total = 361
        offset += 32;

        // 7. Impulse Train (580-D)
        if flags.use_impulse_train {
            if let Some(it) = &self.impulse_train {
                for (i, &val) in it.iter().enumerate() {
                    features[offset + i] = val;
                    mask[offset + i] = 1.0;
                }
            }
        }
        // offset += 580; Total = 941
"""
text = text.replace('// offset += 32; Total = 361', extract_impulse)
text = text.replace('TensorData::new(features, [361])', 'TensorData::new(features, [941])')
text = text.replace('TensorData::new(mask, [361])', 'TensorData::new(mask, [941])')

with open('src/ml/modular_features.rs', 'w') as f:
    f.write(text)
