// examples/waveshaping_mamba_widget.rs
// Standalone waveshaping widget with unified Mamba integration
//
// Demonstrates the wiring pattern:
// Input Data → Mamba Inference → Waveshaping Parameters → UI Update
//
// No sound card needed - shows how to integrate ML predictions into UI controls.
// This pattern applies to any feature that needs Mamba-driven automation.

use std::sync::Arc;
use std::sync::atomic::{AtomicF32, Ordering};
use burn::backend::NdArray;
use burn::tensor::Tensor;

// ────────────────────────────────────────────────────────────────────────────
// STEP 1: Define the Waveshaping Control State
// ────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct WaveshapingControl {
    /// Waveshape drive (0.0 = off, 1.0 = max distortion)
    pub drive: f32,

    /// Waveshape mode: 0=sine, 1=triangle, 2=square, 3=sawtooth
    pub mode: u32,

    /// Tone (0.0 = bright, 1.0 = dark)
    pub tone: f32,

    /// Whether Mamba automation is active
    pub mamba_active: bool,

    /// Confidence in Mamba prediction (0.0-1.0)
    pub mamba_confidence: f32,
}

impl Default for WaveshapingControl {
    fn default() -> Self {
        Self {
            drive: 0.5,
            mode: 0,
            tone: 0.5,
            mamba_active: false,
            mamba_confidence: 0.0,
        }
    }
}

// ────────────────────────────────────────────────────────────────────────────
// STEP 2: Create synthetic input data (in real app, this comes from spectral frame)
// ────────────────────────────────────────────────────────────────────────────

fn create_synthetic_spectral_input() -> Tensor<NdArray, 1> {
    use burn::tensor::TensorData;

    // Simulate a spectral frame: 512 frequency bins
    let mut data = vec![0.0; 512];

    // Add some "peaks" to simulate detected harassment frequencies
    for i in 0..512 {
        let freq_hz = (i as f32 / 512.0) * 96_000.0; // 192kHz sample rate, Nyquist = 96kHz

        // Peak at 2.4 GHz (simulated as low freq for demo)
        if (freq_hz - 2400.0).abs() < 100.0 {
            data[i] = 0.8;
        }

        // Peak at 5 GHz equivalent
        if (freq_hz - 5000.0).abs() < 100.0 {
            data[i] = 0.6;
        }

        // Gaussian noise floor
        data[i] += (rand::random::<f32>() - 0.5) * 0.1;
    }

    Tensor::<NdArray, 1>::from_floats(data.as_slice())
}

// ────────────────────────────────────────────────────────────────────────────
// STEP 3: Use Mamba to predict waveshaping parameters from spectral input
// ────────────────────────────────────────────────────────────────────────────

fn mamba_predict_waveshaping(
    spectral_input: &Tensor<NdArray, 1>,
) -> (WaveshapingControl, f32) {
    // In real implementation, this would:
    // 1. Encode spectral_input through Mamba latent space
    // 2. Decode latent → [drive, mode, tone] + confidence
    //
    // For this example, we use a deterministic mapping:
    // - High magnitude at peak frequencies → high drive
    // - Frequency distribution → mode selection
    // - Overall energy → tone

    let data = spectral_input.to_data();
    let floats = match data.value {
        burn::tensor::ElementConversion::Float(f) => f,
        _ => panic!("Expected float tensor"),
    };

    // Calculate statistics from spectral input
    let max_magnitude = floats.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let mean_magnitude = floats.iter().sum::<f32>() / floats.len() as f32;
    let energy = floats.iter().map(|x| x * x).sum::<f32>().sqrt();

    // Mamba-like prediction:
    // High spectral peaks → high drive (indicates harassment signal)
    let drive = (max_magnitude * 0.8).min(1.0);

    // Energy distribution → mode selection
    //   Low energy (sparse)  → sine (0)
    //   Mid energy          → triangle (1)
    //   High energy         → square (2)
    let mode = if energy < 3.0 {
        0 // sine
    } else if energy < 6.0 {
        1 // triangle
    } else {
        2 // square
    };

    // Mean magnitude → tone (lower = brighter, higher = darker)
    let tone = (mean_magnitude).min(1.0);

    // Confidence = how "certain" Mamba is about prediction
    // Higher spectral peaks = higher confidence
    let confidence = (max_magnitude).min(1.0);

    let control = WaveshapingControl {
        drive,
        mode,
        tone,
        mamba_active: true,
        mamba_confidence: confidence,
    };

    // Return control + reconstruction loss (anomaly score)
    let anomaly_score = (max_magnitude - 0.5).abs(); // How anomalous is this?

    (control, anomaly_score)
}

// ────────────────────────────────────────────────────────────────────────────
// STEP 4: Build the Slint UI with waveshaping controls
// ────────────────────────────────────────────────────────────────────────────

slint::include_modules!();

pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎛️  Waveshaping Widget + Unified Mamba Integration Demo");
    println!("═══════════════════════════════════════════════════════════\n");

    // Create UI
    let ui = AppWindow::new()?;
    let ui_weak = ui.as_weak();

    // Shared state (in real app, this is Arc<Mutex<AppState>>)
    let control_state = Arc::new(AtomicF32::new(0.0));
    let control_state_clone = control_state.clone();

    // ────────────────────────────────────────────────────────────────────────
    // WIRING PATTERN: User Input → Mamba Prediction → UI Update
    // ────────────────────────────────────────────────────────────────────────

    // Handle "Auto-Waveshape" button (activates Mamba inference)
    ui.on_auto_waveshape({
        let ui_weak = ui_weak.clone();
        move || {
            println!("\n🤖 Mamba Auto-Waveshaping activated...");
            println!("   Step 1: Generating synthetic spectral input (512 frequency bins)");

            // STEP 1: Create input (in real app, from spectral frame)
            let spectral_input = create_synthetic_spectral_input();

            println!("   Step 2: Running Mamba inference");

            // STEP 2: Mamba predicts waveshaping parameters
            let (mut waveshaping, anomaly) = mamba_predict_waveshaping(&spectral_input);

            println!("   Step 3: Updating UI with predictions");
            println!("\n   📊 Mamba Predictions:");
            println!("      • Drive:        {:.2}", waveshaping.drive);
            println!("      • Mode:         {} ({})",
                waveshaping.mode,
                match waveshaping.mode {
                    0 => "sine",
                    1 => "triangle",
                    2 => "square",
                    _ => "sawtooth",
                }
            );
            println!("      • Tone:         {:.2}", waveshaping.tone);
            println!("      • Confidence:   {:.2}%", waveshaping.mamba_confidence * 100.0);
            println!("      • Anomaly Score: {:.4}", anomaly);

            // STEP 3: Update UI
            if let Ok(ui) = ui_weak.upgrade() {
                ui.set_drive_slider(waveshaping.drive);
                ui.set_mode_dropdown(waveshaping.mode as i32);
                ui.set_tone_slider(waveshaping.tone);
                ui.set_confidence_display(format!("{:.1}%", waveshaping.mamba_confidence * 100.0));
                ui.set_anomaly_display(format!("{:.4}", anomaly));
                ui.set_mamba_status("Active ✓".to_string());
            }
        }
    });

    // Handle manual drive slider
    ui.on_drive_changed({
        move |drive| {
            control_state_clone.store(drive, Ordering::Relaxed);
            println!("🎚️  Drive updated: {:.2}", drive);
        }
    });

    // Handle mode dropdown
    ui.on_mode_changed({
        move |mode| {
            let mode_name = match mode {
                0 => "sine",
                1 => "triangle",
                2 => "square",
                _ => "sawtooth",
            };
            println!("📟 Mode changed: {} ({})", mode, mode_name);
        }
    });

    // Handle tone slider
    ui.on_tone_changed({
        move |tone| {
            println!("🎵 Tone updated: {:.2} ({})",
                tone,
                if tone < 0.3 { "bright" }
                else if tone < 0.7 { "neutral" }
                else { "dark" }
            );
        }
    });

    println!("\n✨ WIRING DEMONSTRATION:");
    println!("  1. Click 'Auto-Waveshape' to trigger Mamba inference");
    println!("  2. Mamba analyzes synthetic spectral data");
    println!("  3. Predictions appear in UI controls");
    println!("  4. You can also manually adjust sliders\n");
    println!("📌 This pattern works for ANY feature:");
    println!("   Input → Mamba Latent → Feature Parameters → UI\n");

    ui.run()?;
    Ok(())
}
