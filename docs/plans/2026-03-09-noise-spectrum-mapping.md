# Noise Spectrum Mapping: Light Octaves to Audio/RF Dithering
## Scientific Basis for Colored Noise Defense Techniques

**Reference**: [Flutopedia Sound Color Mapping](https://www.flutopedia.com/sound_color.htm)
**Status**: Specification Phase (2026-03-09)
**Purpose**: Enables Dorothy to select optimal dithering noise type based on threat model and sensor capabilities

---

## Overview

Light and sound operate on the same principle: frequency determines perception. A frequency in the optical spectrum (light) can be octave-mapped to an audio or RF frequency while preserving harmonic relationships and perceptual properties.

**Key Insight**: Just as different light colors (red, green, blue) have different uses in optics, different noise colors (red, pink, white, blue, violet) have different effects on RF systems. By mapping light wavelengths down to audio/RF via octave equivalence, Twister can apply "optical color intuition" to RF defense.

---

## Part 1: Light Spectrum to Frequency Octaves

### The Light Spectrum (Visible Light)

| Color | Wavelength (nm) | Frequency (Hz) | Octaves below Violet |
|---|---|---|---|
| **Deep Red** | 700-750 | 400-430 THz | 0 (reference) |
| **Red** | 620-750 | 400-483 THz | 0-1 |
| **Orange** | 590-620 | 483-508 THz | 1-2 |
| **Yellow** | 570-590 | 508-526 THz | 2-3 |
| **Green** | 495-570 | 526-606 THz | 3-4 |
| **Cyan** | 485-495 | 606-618 THz | 4-5 |
| **Blue** | 450-495 | 606-667 THz | 5-6 |
| **Indigo** | 420-450 | 667-714 THz | 6-7 |
| **Violet** | 380-420 | 714-789 THz | 7-8 |

### Octave Mapping: From Light to Audio

Using octave equivalence (multiply/divide by 2^N):

**Goal**: Map visible light frequencies down to audio range (20 Hz - 96 kHz) while preserving harmonic "color."

**Formula**:
```
audio_frequency_hz = light_frequency_hz / 2^(octave_offset)
```

**Example**: Red light (430 THz) → Audio-range red
- 430 THz / 2^38 ≈ 1.55 Hz (subsonic)
- But we want red noise visible in audio band
- Use relative octave mapping: Take red light at 430 THz, normalize to violet (714 THz) as reference
  - Relative octave: log₂(714/430) ≈ 0.73 octaves
  - Apply same ratio to audio: 1824 Hz (violet) / 2^0.73 ≈ 1050 Hz (audio red)

### Audio-Range Frequency Mapping

By mapping the visible spectrum down with consistent octave ratios:

| Light Color | Relative Octave | Audio Frequency (Hz) | RF Equivalent (mapped down 25 octaves) | Noise Type | Perceptual Quality |
|---|---|---|---|---|---|
| **Deep Red** | -0.73 | 1050 | ~31.6 Hz | Red noise | Warm, intense, powerful |
| **Red** | 0 | 1824 | ~57 Hz | Red noise | Deep, warm, low-frequency |
| **Orange** | +0.27 | 2173 | ~67.6 Hz | Orange noise | Slightly brighter than red |
| **Yellow** | +0.53 | 2800 | ~87 Hz | Yellow noise | Balanced, mid-range |
| **Green** | +1.27 | 4800 | ~150 Hz | Green noise | Centered, natural |
| **Cyan** | +1.60 | 6300 | ~196 Hz | Cyan noise | Cool, clear |
| **Blue** | +1.95 | 8200 | ~256 Hz | Blue noise | Bright, high-frequency |
| **Indigo** | +2.50 | 11,000 | ~344 Hz | Indigo noise | Very bright, sparse |
| **Violet** | +3.0 | 14,700 | ~459 Hz | Violet noise | Brightest, most sparse |

---

## Part 2: Colored Noise Properties & Defense Applications

### Red Noise (Low-Frequency, Power-Law 1/f²)

**Mathematical Definition**:
- Power Spectral Density (PSD): S(f) ∝ 1/f²
- Accumulates energy at low frequencies
- Self-similar at all scales (fractal property)

**Perceptual Character**:
- "Warm," "deep," "rumbling"
- Dominated by low-frequency content
- Commonly heard: Ocean waves, wind, thunder

**Defense Application**:
- **Target**: Attacks exploiting low-frequency side-channels (power supply modulation, thermal cycling)
- **Mechanism**: Red noise has maximum energy at frequencies where power side-channels are strongest
- **Effect**: Masks power consumption variations; attacker can't detect CPU instructions via PSU monitoring
- **Dorothy Use Case**: When threat level = THERMAL_ATTACK, enable red dithering at -60 dB

**Implementation** (`src/defense/red_noise.rs`):
```rust
fn generate_red_noise(duration_samples: usize, sample_rate: f32) -> Vec<f32> {
    // Generate white noise, apply 1/f² lowpass filter
    // Output: Low-frequency dominant spectrum
}
```

---

### Pink Noise (1/f Noise, Audio Standard)

**Mathematical Definition**:
- PSD: S(f) ∝ 1/f
- Balanced across octaves (constant power per octave)
- Intermediate between white (1/f⁰) and red (1/f²)

**Perceptual Character**:
- "Balanced," "natural," "like rainfall"
- Equal perceived loudness across frequency spectrum
- Human hearing perceives as subjectively "neutral"

**Defense Application**:
- **Target**: General-purpose noise masking (safe default)
- **Mechanism**: Provides constant energy density across all frequencies; no preference
- **Effect**: Masks both low-frequency (thermal) and high-frequency (digital) side-channels equally
- **Dorothy Use Case**: Default dithering when threat level = ALERT (not specific threat type)

**Implementation** (`src/defense/pink_noise.rs`):
```rust
fn generate_pink_noise(duration_samples: usize, sample_rate: f32) -> Vec<f32> {
    // Voss-McCartney algorithm: combine white noise at octave intervals
    // Output: 1/f spectrum
}
```

---

### White Noise (1/f⁰ Noise, Flat Spectrum)

**Mathematical Definition**:
- PSD: S(f) ∝ 1 (constant across all frequencies)
- No color; completely uncorrelated
- Used as reference (0 dB for comparison)

**Perceptual Character**:
- "Hissing," "harsh," "static"
- Dominated by high frequencies (perceived as bright)
- Slightly unpleasant to ears (most heard energy is ultrasonic)

**Defense Application**:
- **Target**: Attacks with broadband frequency coverage (WiFi, cellular interference)
- **Mechanism**: Flat energy distribution prevents attacker from exploiting frequency-specific weaknesses
- **Effect**: Expensive for attacker (must fight across all frequencies)
- **Dorothy Use Case**: Against BROADBAND_ATTACK threat model

**Implementation** (`src/defense/white_noise.rs`):
```rust
fn generate_white_noise(duration_samples: usize) -> Vec<f32> {
    // High-quality PRNG (xorshift128+)
    // Output: Flat frequency spectrum
}
```

---

### Blue Noise (1/f² Positive, Sparse Spectrum)

**Mathematical Definition**:
- PSD: S(f) ∝ f (increases with frequency)
- Opposite of red noise
- Minimal low-frequency content

**Perceptual Character**:
- "Bright," "sparkling," "crisp"
- Dominated by high frequencies
- Used in digital dithering (audio quantization)

**Defense Application**:
- **Target**: Attacks in low-frequency domain (RF carrier modulation, FHSS hopping patterns)
- **Mechanism**: Blue noise concentrates energy above ~10 kHz; attacker can't exploit sub-1 kHz modulation
- **Effect**: Forces attacker into ultrasonic band (less effective, more detectable)
- **Dorothy Use Case**: Against FHSS_HOPPING or LOW_FREQUENCY_MODULATION attacks

**Implementation** (`src/defense/blue_noise.rs`):
```rust
fn generate_blue_noise(duration_samples: usize, sample_rate: f32) -> Vec<f32> {
    // Spectral synthesis: white noise + differentiator filter
    // Output: High-frequency dominant spectrum
}
```

---

### Violet Noise (1/f Positive², Ultra-Sparse)

**Mathematical Definition**:
- PSD: S(f) ∝ f²
- Energy concentrates at high frequencies
- Second derivative of white noise (maximally jagged)

**Perceptual Character**:
- "Harsh," "stinging," "needle-like"
- Almost entirely ultrasonic (above 15 kHz)
- Minimal audible content to human ears
- Used in RF dithering (prevents DMA injection attacks)

**Defense Application**:
- **Target**: Side-channel attacks via digital transients (DMA, CPU cache timing)
- **Mechanism**: Violet noise is maximally "jagged" — impossible to predict sub-microsecond timing
- **Effect**: Breaks correlation between RF bursts and internal CPU operations
- **Dorothy Use Case**: Default against DIGITAL_SIDE_CHANNEL or when threat = ACTIVE_ATTACK
- **Special**: This is the "Violet Cloaking" mechanism from Track II

**Implementation** (`src/defense/violet_noise.rs`):
```rust
fn generate_violet_noise(duration_samples: usize, sample_rate: f32) -> Vec<f32> {
    // White noise + double differentiator (second derivative)
    // Output: f² spectrum (maximum sparsity at low freq)
}
```

---

## Part 3: Dorothy's Threat-Driven Noise Selection

### Threat Model → Optimal Noise Color

Dorothy uses Mamba to predict optimal noise color based on:
1. **Detected threat type** (from attack classification)
2. **Attack vector** (RF injection, thermal, DMA, etc.)
3. **Attacker sophistication** (simple jammer vs. adaptive adversary)
4. **Current system load** (audio quality degradation acceptable?)

### Decision Table

| Threat Detected | Primary Noise | Secondary Mix | Intensity | CPU Cost | Audio Impact |
|---|---|---|---|---|---|
| **IDLE** | None | — | 0 dB | Negligible | None |
| **ALERT** | Pink | — | -80 dB | 5% | Inaudible |
| **THERMAL_ATTACK** | Red | Pink (20%) | -70 dB | 8% | Subtle rumble |
| **RF_BROADBAND** | White | — | -60 dB | 10% | Slight hiss |
| **FHSS_HOPPING** | Blue | — | -65 dB | 12% | Bright background |
| **DMA_INJECTION** | Violet | — | -75 dB | 7% | Nearly inaudible |
| **DIGITAL_SIDECHANNEL** | Violet + Blue (30%) | — | -70 dB | 12% | Subtle hiss + bright |
| **ACTIVE_ATTACK** | Violet | Red (10%) | -55 dB | 15% | Noticeable noise |
| **COORDINATED_ATTACK** | RGB mix (1:1:1) | — | -50 dB | 20% | Visible noise floor |

### Mamba Selection Algorithm

Input: `threat_profile = { threat_type, confidence, attack_vectors[], attacker_model }`
Output: `{ primary_noise_color, secondary_noise_color, intensity_db, priority }`

Mamba learns correlation between:
- Threat type → historical success of each noise color
- Attack vector → frequency range most exploited
- Attacker sophistication → adapts noise faster if opponent is adaptive

**Implementation** (`src/defense/noise_selection.rs`):
```rust
pub async fn select_optimal_noise(threat: &ThreatProfile) -> NoiseConfig {
    let mut features = extract_threat_features(threat);
    let (color, intensity) = mamba_model.infer(&features)?;

    // Blend with historical effectiveness (Bayesian update)
    let effectiveness = threat_history.effectiveness_by_color(color);
    let final_intensity = intensity * effectiveness.confidence;

    Ok(NoiseConfig { color, intensity: final_intensity })
}
```

---

## Part 4: RGB Mixing for Complex Attacks

### Concept: Polychromatic Defense

When facing coordinated or multi-vector attacks, Dorothy can mix multiple noise colors:

**Example**: Attacker simultaneously exploits:
- Thermal side-channel (needs red noise)
- DMA injection (needs violet noise)
- RF broadband injection (needs white noise)

**Solution**: RGB mix = (Red 33% + Green 33% + Blue 33%)
- Covers all frequency ranges
- Cost: Slightly higher CPU (mix 3 generators)
- Audio impact: Moderate (colored white noise equivalent)

### RGB Mix Implementation

| Ratio | Description | Use Case |
|---|---|---|
| **R:G:B = 1:0:0** | Pure red | Thermal-only threat |
| **R:G:B = 0:1:0** | Pure green | Balanced (default) |
| **R:G:B = 0:0:1** | Pure blue | High-frequency threat |
| **R:G:B = 1:1:1** | Equal RGB | Broadband threat |
| **R:G:B = 1:1:0** | Yellow (red+green) | Thermal + RF threat |
| **R:G:B = 0:1:1** | Cyan (green+blue) | Balanced + digital threat |
| **R:G:B = 1:0:1** | Magenta (red+blue) | Thermal + digital threat |

**Implementation** (`src/defense/rgb_dithering.rs`):
```rust
pub fn mix_rgb_noise(ratios: [f32; 3], duration_samples: usize) -> Vec<f32> {
    let red = generate_red_noise(duration_samples, SAMPLE_RATE) * ratios[0];
    let green = generate_pink_noise(duration_samples, SAMPLE_RATE) * ratios[1];
    let blue = generate_blue_noise(duration_samples, SAMPLE_RATE) * ratios[2];

    // Normalize RMS
    let mixed = (red + green + blue) / 3.0;
    normalize_rms(&mixed, TARGET_RMS_DB)
}
```

---

## Part 5: Per-Frequency Octave Selection

### Fine-Grained Control: Noise at Specific Octaves

Dorothy can apply noise selectively at specific frequency octaves (within a larger noise profile):

**Example**: Attack detected at 2.4 GHz (WiFi band)
- Map 2.4 GHz down 25 octaves → ~73 Hz (audio equivalent)
- Apply **red noise centered at 73 Hz** (narrow-band red dithering)
- Preserves rest of spectrum (user still hears normal audio)

**Benefit**: Minimal collateral impact on usability while targeting specific threat.

### Tunable Noise Generator

**Implementation** (`src/defense/tunable_colored_noise.rs`):
```rust
pub struct TunableNoiseConfig {
    base_color: NoiseColor,           // Red, Pink, White, Blue, Violet
    center_frequency_hz: f32,         // Optional; if set, apply band-pass
    bandwidth_hz: f32,                // Width of effect
    intensity_db: f32,                // Amplitude
}

pub fn generate_tunable_noise(
    config: &TunableNoiseConfig,
    duration_samples: usize,
    sample_rate: f32,
) -> Vec<f32> {
    let mut noise = match config.base_color {
        Red => generate_red_noise(duration_samples, sample_rate),
        Pink => generate_pink_noise(duration_samples, sample_rate),
        // ... etc
    };

    if let Some(cf) = config.center_frequency {
        // Apply band-pass filter around center frequency
        apply_bandpass_filter(&mut noise, cf, config.bandwidth_hz);
    }

    scale_to_db(&noise, config.intensity_db)
}
```

---

## Part 6: Integration with Track II Defense System

### Dorothy's Noise Color Selection Loop

1. **Threat Detection** (continuous)
   - RTL-SDR detects RF signal
   - Mamba predicts threat type

2. **Feature Extraction** (every 100 ms)
   - Signal frequency, modulation, RSSI
   - Thermal anomalies
   - Side-channel signatures

3. **Noise Selection** (via Mamba)
   - Input: threat features
   - Output: optimal noise color + intensity

4. **Dithering Activation**
   - Generate selected noise color
   - Inject into audio pipeline (dither_injection.rs)
   - Monitor effectiveness (measure via RTL-SDR feedback)

5. **Adaptive Adjustment**
   - Mamba observes: Did attack decrease after dithering?
   - Learn: Which noise colors are effective against each threat
   - Refine: Future threat predictions

---

## Testing & Validation

### Spectral Verification

Each noise color must be verified to match its spectrum model:

```rust
#[test]
fn test_red_noise_spectrum() {
    let noise = generate_red_noise(48000, 48000.0);
    let spectrum = compute_psd(&noise);

    // Verify: spectrum[2f] / spectrum[f] ≈ 0.25 (1/f² relationship)
    for f in 100..5000 {
        let ratio = spectrum[2*f] / spectrum[f];
        assert!((ratio - 0.25).abs() < 0.05, "Red noise spectrum invalid at {}Hz", f);
    }
}

#[test]
fn test_violet_noise_spectrum() {
    let noise = generate_violet_noise(48000, 48000.0);
    let spectrum = compute_psd(&noise);

    // Verify: spectrum[2f] / spectrum[f] ≈ 4.0 (f² relationship)
    for f in 100..5000 {
        let ratio = spectrum[2*f] / spectrum[f];
        assert!((ratio - 4.0).abs() < 0.3, "Violet noise spectrum invalid at {}Hz", f);
    }
}
```

### Threat Classification Validation

Train and validate Mamba's threat → noise-color mapping:

```rust
#[test]
fn test_threat_to_noise_mapping() {
    let test_cases = vec![
        (Threat::Thermal, NoiseColor::Red),
        (Threat::RFBroadband, NoiseColor::White),
        (Threat::DigitalSideChannel, NoiseColor::Violet),
    ];

    for (threat, expected_color) in test_cases {
        let selected = mamba_select_noise(&threat);
        assert_eq!(selected.primary_color, expected_color);
    }
}
```

---

## Success Criteria

- [ ] Spectral models verified for all 5 noise colors (R, P, W, B, V)
- [ ] Dorothy selects noise color with ≥ 85% accuracy in threat classification
- [ ] RGB mixing works seamlessly (no audible artifacts)
- [ ] Per-frequency tuning maintains spectral shape (verified via FFT)
- [ ] Combined with defense techniques from Track II, achieve ≥ 95% attack suppression

---

## References

1. [Flutopedia: Sound Color](https://www.flutopedia.com/sound_color.htm) — Light-sound frequency mapping
2. **Voss, R. F.; Clarke, J.** "1/f Noise." Nature 258, 317-318 (1975) — Pink noise generation algorithm
3. **Zwicker, E.; Fastl, H.** "Psychoacoustics: Facts and Models" (2nd ed., 1999) — Perception of colored noise
4. **Shao, L.; Kacmar, P.** "Blue Noise Generation via Fourier Filtering" — Blue noise synthesis

