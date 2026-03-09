import re

with open("src/audio.rs", "r") as f:
    content = f.read()

# Add fft_to_mel_scale
if "pub fn fft_to_mel_scale" not in content:
    content += """

pub fn fft_to_mel_scale(fft_512: &[f32; 512]) -> [f32; 128] {
    let mut mel = [0.0f32; 128];
    // Simple linear decimation for mock mel-scale
    for i in 0..128 {
        let mut sum = 0.0;
        for j in 0..4 {
            sum += fft_512[i * 4 + j];
        }
        mel[i] = sum / 4.0;
    }
    mel
}

pub fn compute_bispectrum(fft_512: &[f32; 512]) -> [f32; 64] {
    let mut bispec = [0.0f32; 64];
    for i in 0..64 {
        if i < fft_512.len() {
            bispec[i] = fft_512[i]; // Mock implementation
        }
    }
    bispec
}
"""

with open("src/audio.rs", "w") as f:
    f.write(content)
