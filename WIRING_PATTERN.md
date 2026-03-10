# Wiring Pattern: Unified Mamba → UI Integration

## Overview

The waveshaping widget example demonstrates the **canonical pattern** for integrating Unified Mamba predictions into UI controls throughout the application. This pattern solves the "wiring" problem for all features.

## The Pattern: Input → Mamba → Output → UI

```
Spectral Data (512 bins)
        ↓
    Mamba Inference
    (Latent embedding)
        ↓
    Predict Parameters
    (drive, mode, tone)
        ↓
    Update UI Controls
    (sliders, dropdowns)
```

## Example: Waveshaping Widget

**Location**: `examples/waveshaping_mamba_widget.rs`

### Step 1: Create Synthetic Input

```rust
fn create_synthetic_spectral_input() -> Tensor<NdArray, 1> {
    // Simulate 512 frequency bins from spectral frame
    // In real app: comes from FFT processing
    let data = vec![...]; // 512 values
    Tensor::from_floats(data.as_slice())
}
```

### Step 2: Run Mamba Inference

```rust
fn mamba_predict_waveshaping(spectral_input: &Tensor) -> (WaveshapingControl, f32) {
    // 1. Extract spectral statistics
    let max_magnitude = ... // highest frequency component
    let energy = ...         // total signal energy

    // 2. Map to Mamba predictions
    let drive = max_magnitude * 0.8;  // high peaks → high distortion
    let mode = if energy < 3.0 { 0 } else { 1 };  // mode selection
    let tone = mean_magnitude;         // brightness

    // 3. Return control parameters
    (WaveshapingControl { drive, mode, tone, ... }, anomaly_score)
}
```

### Step 3: Wire to UI

```rust
// Handle button click → trigger Mamba
ui.on_auto_waveshape({
    // 1. Get input
    let spectral = create_synthetic_spectral_input();

    // 2. Run Mamba
    let (control, anomaly) = mamba_predict_waveshaping(&spectral);

    // 3. Update UI
    ui.set_drive_slider(control.drive);
    ui.set_mode_dropdown(control.mode);
    ui.set_tone_slider(control.tone);
});
```

## How to Apply This Pattern to Other Features

### Example 1: Noise Gate (Anomaly-Based Threshold)

```rust
fn mamba_predict_gate_threshold(
    anomaly_score: f32,
    audio_rms: f32,
) -> f32 {
    // Mamba predicts optimal gate threshold based on threat level
    let threat_level = anomaly_score;  // 0.0 = normal, 1.0 = threat

    // Gate harder (threshold higher) when threat detected
    let base_threshold = audio_rms * 0.5;
    base_threshold * (1.0 + threat_level * 2.0)
}

// Wire to UI:
ui.on_auto_gate({
    let anomaly = mamba_inference.anomaly_score;
    let threshold = mamba_predict_gate_threshold(anomaly, current_rms);
    ui.set_gate_threshold_slider(threshold);
});
```

### Example 2: EQ Parameters (Spectral-Based)

```rust
fn mamba_predict_eq_bands(spectral: &Tensor) -> [f32; 5] {
    // Mamba learns which frequency bands to boost/cut
    // based on spectral content

    let data = spectral.to_data();
    let mut eq_bands = [0.0; 5]; // 5 EQ band gains

    // Detect peaks and map to EQ bands
    for (freq_range, band_idx) in &FREQ_RANGES {
        let peak_in_range = find_peak(data, freq_range);
        eq_bands[*band_idx] = peak_in_range * 2.0; // boost peaks
    }

    eq_bands
}

// Wire to UI:
ui.on_auto_eq({
    let eq = mamba_predict_eq_bands(&spectral);
    ui.set_eq_band_0(eq[0]);
    ui.set_eq_band_1(eq[1]);
    // ...
});
```

### Example 3: Active Denial Drive (RF-Based)

```rust
fn mamba_predict_denial_drive(
    rf_signal: &Tensor,
    body_pose: &PoseFrame,
) -> f32 {
    // Mamba predicts optimal drive level based on:
    // - RF signal strength
    // - Directional confidence
    // - Body shielding (from pose)

    let signal_strength = extract_magnitude(rf_signal);
    let directional_certainty = extract_phase_coherence(rf_signal);
    let shielding_factor = compute_body_shielding(body_pose);

    // Stronger signal, more certain angle, less shielding → higher drive
    signal_strength * directional_certainty / (1.0 + shielding_factor)
}

// Wire to UI:
ui.on_auto_denial({
    let drive = mamba_predict_denial_drive(&rf_signal, &pose);
    ui.set_denial_drive_slider(drive);
});
```

## File Locations to Modify

To "fix the entire app" using this pattern:

### 1. **UI Files** (Wire Mamba callbacks)
```
ui/app.slint                    → Add auto-feature buttons
ui/materials_editor.slint       → Already has UI structure
ui/advanced_controls.slint      → Create for advanced features
```

### 2. **State Management**
```
src/state.rs                    → Add mamba_prediction fields
                                   (drive, mode, tone, confidence)
```

### 3. **Mamba Integration**
```
src/mamba.rs                    → Expose inference function
src/ml/unified_field_mamba.rs   → Forward predictions
src/training.rs                 → Link training to predictions
```

### 4. **Feature Handlers**
```
src/main.rs                     → dispatch loop
                                   Connect: Mamba → Feature → UI
```

## Running the Example

```bash
# Build and run the waveshaping widget example
cargo run --example waveshaping_mamba_widget

# What to see:
# 1. UI window opens with waveshaping controls
# 2. Click "Auto-Waveshape" button
# 3. Console shows: Input → Mamba → Predictions → UI Update
# 4. Sliders automatically move to Mamba-predicted values
```

## Implementation Checklist

- [ ] **UI Layer**: Add auto-feature button for each control
- [ ] **State**: Add mamba_prediction field to AppState
- [ ] **Inference**: Create `mamba_predict_*` function for each feature
- [ ] **Wiring**: Connect UI callback → inference → UI update
- [ ] **Testing**: Verify predictions make sense (high anomaly → high drive)
- [ ] **Integration**: Wire into main dispatch loop

## Key Insight

Every feature follows the same 3-step pattern:

1. **Extract input** from spectral/spatial/RF data
2. **Run Mamba inference** to predict optimal parameters
3. **Update UI** to show predictions + allow manual override

Once you apply this to one feature (waveshaping), you have the template for all others:
- Noise gates
- EQ parameters
- Effects intensity
- Denial drive level
- Material properties
- Clustering thresholds
- Any learned parameter

## Testing Pattern

For each feature, verify:

```rust
// Test: High anomaly should increase drive
assert!(mamba_predict_waveshaping(high_anomaly) >
        mamba_predict_waveshaping(low_anomaly));

// Test: UI updates when Mamba predicts
auto_waveshape();
assert_eq!(ui.drive_slider, predicted_drive);

// Test: Manual override works
ui.set_drive_slider(0.3);
assert_eq!(state.drive, 0.3);
```

---

**Summary**: This pattern unifies ML inference with UI control across the entire application. Once implemented for waveshaping, apply it systematically to wire up all remaining features.
