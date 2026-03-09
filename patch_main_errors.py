import re

with open("src/main.rs", "r") as f:
    content = f.read()

# Fix repeated fields
pattern_duplicate_fields = r"""                impulse_detection: None,
                video_frame: None,
                video_frame_timestamp_us: 0,
                visual_features: None,
                anc_phase: None,
                harmonic_energy: None,
                impulse_detection: None,
                video_frame: None,
                video_frame_timestamp_us: 0,
                visual_features: None,"""

replacement_fields = """                impulse_detection: None,
                video_frame: None,
                video_frame_timestamp_us: 0,
                visual_features: None,
                anc_phase: None,
                harmonic_energy: None,"""

content = content.replace(pattern_duplicate_fields, replacement_fields)

# Fix SpectralFrame confidence initialization
pattern_spectral = r"""                        let frame = crate::ml::spectral_frame::SpectralFrame {
                            timestamp_micros: chrono::Utc::now\(\)\.timestamp_micros\(\) as u64,
                            fft_magnitude: fft_mag,
                            bispectrum: \[0\.0; 64\], // Populated later if needed
                            itd_ild: \[0\.0; 4\],
                            beamformer_outputs: \[0\.0; 3\],
                            mamba_anomaly_score: anomaly,
                        };"""

replacement_spectral = """                        let frame = crate::ml::spectral_frame::SpectralFrame {
                            timestamp_micros: chrono::Utc::now().timestamp_micros() as u64,
                            fft_magnitude: fft_mag,
                            bispectrum: [0.0; 64], // Populated later if needed
                            itd_ild: [0.0; 4],
                            beamformer_outputs: [0.0; 3],
                            mamba_anomaly_score: anomaly,
                            confidence: 1.0,
                        };"""

content = re.sub(pattern_spectral, replacement_spectral, content)

with open("src/main.rs", "w") as f:
    f.write(content)
