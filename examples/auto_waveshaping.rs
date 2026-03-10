/// examples/auto_waveshaping.rs
/// Live Neural Waveshaping Micro-App
///
/// The applet goal: deliver unburdened examples for continuous integration
/// Implements the complete pipeline:
/// 1. UnifiedFieldMamba analyzes incoming signal
/// 2. project_latent_to_waveshape converts 128D latent → parameters
/// 3. Neural waveshape is applied in-place to TX buffer
/// 4. SVG oscilloscope renders at 144 Hz
///
/// Run with: cargo run --example auto_waveshaping

use tokio::time::{interval, Duration};
use slint::SharedString;

use twister::ml::unified_field_mamba::UnifiedFieldMamba;
use twister::dispatch::stream_packer::GpuStreamPacker;
use twister::ml::waveshape_projection::{project_latent_to_waveshape, NeuralWaveshapeParams};
use burn::tensor::Tensor;
use burn::backend::NdArray;

type Backend = NdArray;

slint::slint! {
    import { VerticalBox, HorizontalBox, Slider, Switch, ProgressIndicator } from "std-widgets.slint";

    export global WaveshapeEngine {
        in-out property <bool> auto-steer: true;
        in-out property <float> anomaly-score: 0.0;
        in-out property <float> drive: 0.0;
        in-out property <float> foldback: 0.0;
        in-out property <float> asymmetry: 0.0;
        in-out property <string> live-waveform-path: "M 0 60 L 600 60";
    }

    export component AutoWaveshapingApplet inherits Window {
        title: "AG-UI: Live Neural Waveshaper";
        width: 600px;
        height: 500px;
        background: #0a0a0a;

        VerticalBox {
            spacing: 15px;
            padding: 20px;

            HorizontalBox {
                spacing: 10px;
                Text {
                    text: "🤖 UNIFIED MAMBA INFERENCE LOOP";
                    color: #0cf;
                    font-weight: 800;
                    font-size: 16px;
                }
                Switch {
                    text: "Neural Auto-Steer";
                    checked <=> WaveshapeEngine.auto-steer;
                }
            }

            // Threat Metric
            HorizontalBox {
                Text { text: "Threat Level:"; color: #aaa; }
                Text {
                    text: round(WaveshapeEngine.anomaly-score * 100) / 100;
                    color: WaveshapeEngine.anomaly-score > 0.5 ? #f00 : #0f0;
                    font-weight: 800;
                }
            }

            // Animated Oscilloscope
            Rectangle {
                height: 120px;
                background: #111;
                border-color: WaveshapeEngine.anomaly-score > 0.5 ? #500 : #050;
                border-width: 2px;
                Path {
                    width: 100%;
                    height: 100%;
                    stroke: #b0f;
                    stroke-width: 2px;
                    commands: WaveshapeEngine.live-waveform-path;
                }
            }

            // Latent Activity Indicators
            Text { text: "128D Latent Activity"; color: #888; font-size: 11px; }
            HorizontalBox {
                spacing: 5px;
                ProgressIndicator {
                    width: 33%;
                    progress: WaveshapeEngine.drive;
                    animate progress { duration: 16ms; easing: ease-in-out; }
                }
                ProgressIndicator {
                    width: 33%;
                    progress: WaveshapeEngine.foldback;
                    animate progress { duration: 16ms; easing: ease-in-out; }
                }
                ProgressIndicator {
                    width: 33%;
                    progress: WaveshapeEngine.asymmetry;
                    animate progress { duration: 16ms; easing: ease-in-out; }
                }
            }

            // Parameter Readouts
            HorizontalBox {
                Text { text: "Drive: " + round(WaveshapeEngine.drive * 100) + "%"; color: #fff; }
                Text { text: "Fold: " + round(WaveshapeEngine.foldback * 100) + "%"; color: #fff; }
                Text { text: "Asym: " + round(WaveshapeEngine.asymmetry * 100) + "%"; color: #fff; }
            }
        }
    }
}

/// Zero-allocation SVG path generation for oscilloscope
fn generate_oscilloscope_path(audio_buffer: &[f32], ui_width: f32, ui_height: f32) -> String {
    let num_samples = audio_buffer.len();
    if num_samples == 0 {
        return String::new();
    }

    let max_points = ui_width as usize;
    let stride = (num_samples / max_points).max(1);
    let estimated_capacity = (num_samples / stride) * 16;
    let mut path = String::with_capacity(estimated_capacity);

    let mid_y = ui_height / 2.0;
    let x_step = ui_width / ((num_samples / stride) as f32).max(1.0);
    let mut point_index = 0;

    for i in (0..num_samples).step_by(stride) {
        let sample = audio_buffer[i];
        let x = point_index as f32 * x_step;
        let clamped_sample = sample.clamp(-1.0, 1.0);
        let y = mid_y - (clamped_sample * mid_y);

        if point_index == 0 {
            path.push_str(&format!("M {:.1} {:.1} ", x, y));
        } else {
            path.push_str(&format!("L {:.1} {:.1} ", x, y));
        }

        point_index += 1;
    }

    path
}

/// In-place neural waveshaping based on Mamba projections
fn apply_neural_waveshape(audio_buffer: &mut [f32], params: &NeuralWaveshapeParams) {
    // Bypass if both drive and asymmetry are minimal
    if params.drive < 0.01 && params.asymmetry.abs() < 0.01 {
        return;
    }

    let drive_multiplier = 1.0 + (params.drive * 20.0);
    let fold_intensity = params.foldback * std::f32::consts::PI;

    for sample in audio_buffer.iter_mut() {
        let mut val = *sample + params.asymmetry;
        val *= drive_multiplier;

        if params.foldback > 0.01 {
            val = (val * fold_intensity).sin();
        } else {
            val = val.clamp(-1.0, 1.0);
        }

        *sample = val;
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ui = AutoWaveshapingApplet::new()?;
    let ui_handle = ui.as_weak();

    // Initialize Mamba and signal packer
    let device = burn::backend::ndarray::NdArrayDevice::default();
    let mamba = UnifiedFieldMamba::<Backend>::new(&device, 128); // 128-D latent embedding
    let mut packer = GpuStreamPacker::new(4096);

    // 100Hz Signal Dispatch Loop
    tokio::spawn(async move {
        let mut tick = interval(Duration::from_millis(10));
        let mut simulated_time: f32 = 0.0;
        let active_sample_rate = 192_000.0;

        loop {
            tick.tick().await;
            simulated_time += 0.01;

            packer.reset_frame();
            let mut cursor = 0;

            // Generate mock hardware audio bytes
            let num_samples = 512;
            let mut mock_pcm = Vec::with_capacity(num_samples * 2);
            for i in 0..num_samples {
                let base_freq = (i as f32 * 0.1 + simulated_time).sin();
                let sweep = if simulated_time.sin() > 0.5 {
                    (i as f32 * 3.0).sin()
                } else {
                    0.0
                };
                let sample_f32 = (base_freq + sweep) * 0.5;
                let sample_i16 = (sample_f32 * 32767.0) as i16;
                mock_pcm.extend_from_slice(&sample_i16.to_le_bytes());
            }

            packer.pack_16bit_stream(&mock_pcm, &mut cursor);

            // Prepare Mamba input tensor [1, 512, 9]
            let mut tensor_data = Vec::with_capacity(512 * 9);
            for i in 0..512 {
                let val = *packer.staging_buffer.get(i).unwrap_or(&0.0);
                for _ in 0..9 {
                    tensor_data.push(val);
                }
            }

            let input_tensor = Tensor::<Backend, 3>::from_data(
                burn::tensor::TensorData::new(tensor_data, vec![1, 512, 9]),
                &device
            );

            // Mamba forward pass - returns enriched tensor (with latent features)
            let output_tensor = mamba.forward(input_tensor.clone());

            // Compute anomaly score (MSE reconstruction loss)
            let diff = output_tensor.clone().sub(input_tensor);
            let mse = diff.clone().mul(diff).mean();
            let anomaly_score: f32 = mse.into_scalar().into();

            // Extract latent from output tensor and project to waveshape parameters
            // Output tensor is [batch, n_particles, latent_dim] = [1, 512, 128]
            let latent_data = output_tensor.mean_dim(1).into_data().to_vec::<f32>().unwrap();
            let mut latent_array = [0.0f32; 128];
            for (i, val) in latent_data.iter().enumerate().take(128) {
                latent_array[i] = *val;
            }

            let params = project_latent_to_waveshape(&latent_array, active_sample_rate);

            // Apply neural waveshape to TX buffer
            let mut tx_buffer: Vec<f32> = packer.staging_buffer.clone();
            apply_neural_waveshape(&mut tx_buffer, &params);

            // Generate SVG oscilloscope path
            let svg_path = generate_oscilloscope_path(&tx_buffer, 600.0, 120.0);

            // Update UI
            let ui_clone = ui_handle.clone();
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(ui) = ui_clone.upgrade() {
                    let backend = ui.global::<WaveshapeEngine>();
                    backend.set_anomaly_score(anomaly_score.clamp(0.0, 1.0));

                    if backend.get_auto_steer() {
                        backend.set_drive(params.drive);
                        backend.set_foldback(params.foldback);
                        backend.set_asymmetry((params.asymmetry + 1.0) / 2.0); // Normalize [-1, 1] to [0, 1]
                    }

                    backend.set_live_waveform_path(SharedString::from(svg_path));
                }
            });
        }
    });

    ui.run()?;
    Ok(())
}
