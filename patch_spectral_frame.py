import re

with open("src/ml/spectral_frame.rs", "r") as f:
    content = f.read()

replacement = """        SpectralFrame {
            timestamp_micros: 0,
            fft_magnitude: [0.0; 128],
            bispectrum: [0.0; 64],
            itd_ild: [0.0; 4],
            beamformer_outputs: [0.0; 3],
            mamba_anomaly_score: 0.0,
            confidence: 0.0,
        }"""

content = content.replace("""        SpectralFrame {
            timestamp_micros: 0,
            fft_magnitude: [0.0; 128],
            bispectrum: [0.0; 64],
            itd_ild: [0.0; 4],
            beamformer_outputs: [0.0; 3],
            mamba_anomaly_score: 0.0,
        }""", replacement)

with open("src/ml/spectral_frame.rs", "w") as f:
    f.write(content)
