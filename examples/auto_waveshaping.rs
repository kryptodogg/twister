slint::include_modules!();
use std::sync::Arc;
use tokio::time::{interval, Duration};
use slint::{Weak, SharedString};
use twister::ml::unified_field_mamba::UnifiedFieldMamba;
use twister::dispatch::stream_packer::GpuStreamPacker;
use twister::ml::waveshape_projection::{project_latent_to_waveshape, NeuralWaveshapeParams};
use burn::tensor::{Tensor, Device};
use burn::backend::Wgpu;

type Backend = burn::backend::NdArray; // Use NdArray to bypass wgpu Send/Sync issues

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ui = AutoWaveshapingApplet::new()?;
    let ui_handle = ui.as_weak();

    // 1. Initialize the Neural Operator and CPU device
    let device = burn::backend::ndarray::NdArrayDevice::default();
    let mamba = UnifiedFieldMamba::<Backend>::new(&device);
    let mut packer = GpuStreamPacker::new(4096);

    // 2. The 100Hz Signal Dispatch Loop
    tokio::spawn(async move {
        let mut tick = interval(Duration::from_millis(10)); // 100Hz
        let mut simulated_time: f32 = 0.0;
        let active_sample_rate = 192_000.0;

        // Helper for simple MSE loss on tensors
        let compute_loss = |output: &Tensor<Backend, 3>, input: &Tensor<Backend, 3>| -> f32 {
            let diff = output.clone().sub(input.clone());
            let mse = diff.clone().mul(diff).mean();
            mse.into_scalar().into()
        };

        loop {
            tick.tick().await;
            simulated_time += 0.1;

            packer.reset_frame();
            let mut cursor = 0;

            // Generate mock hardware bytes
            let num_samples = 512;
            let mut mock_pcm = Vec::with_capacity(num_samples * 2);
            for i in 0..num_samples {
                let base_freq = (i as f32 * 0.1 + simulated_time).sin();
                let sweep = if simulated_time.sin() > 0.5 { (i as f32 * 3.0).sin() } else { 0.0 };
                let sample_f32 = (base_freq + sweep) * 0.5;
                let sample_i16 = (sample_f32 * 32767.0) as i16;
                mock_pcm.extend_from_slice(&sample_i16.to_le_bytes());
            }

            packer.pack_16bit_stream(&mock_pcm, &mut cursor);

            let mut tensor_data = Vec::with_capacity(512 * 9);
            for i in 0..512 {
                let val = *packer.staging_buffer.get(i).unwrap_or(&0.0);
                tensor_data.extend_from_slice(&[
                    val, val, val,
                    val, val,
                    val, val, val,
                    val
                ]);
            }

            let input_tensor = Tensor::<Backend, 3>::from_data(
                burn::tensor::TensorData::new(tensor_data, vec![1, 512, 9]),
                &device
            );

            let (output_tensor, latent_tensor) = mamba.forward(input_tensor.clone());
            let anomaly_score = compute_loss(&output_tensor, &input_tensor);

            let mean_latent = latent_tensor.mean_dim(1).into_data().to_vec::<f32>().unwrap();
            let mut latent_array = [0.0f32; 128];
            for (i, val) in mean_latent.iter().enumerate().take(128) {
                latent_array[i] = *val;
            }

            let params = project_latent_to_waveshape(&latent_array, active_sample_rate);

            let mut path_commands = String::with_capacity(512 * 15);
            path_commands.push_str("M 0 60");
            for (i, val) in packer.staging_buffer.iter().take(512).enumerate() {
                let x = (i as f32 / 512.0) * 600.0;
                let smeared_val = val * (1.0 + params.drive) * (1.0 + params.asymmetry);
                let y = 60.0 - (smeared_val * 40.0);
                path_commands.push_str(&format!(" L {:.1} {:.1}", x, y));
            }

            let ui_clone = ui_handle.clone();
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(ui) = ui_clone.upgrade() {
                    let backend = ui.global::<WaveshapeEngine>();
                    backend.set_anomaly_score(anomaly_score);

                    if backend.get_auto_steer() {
                        backend.set_drive(params.drive);
                        backend.set_foldback(params.foldback);
                        backend.set_asymmetry(params.asymmetry);
                    }

                    backend.set_live_waveform_path(SharedString::from(path_commands));
                }
            });
        }
    });

    ui.run()?;
    Ok(())
}
