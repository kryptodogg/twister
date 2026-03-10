use std::sync::Arc;
use tokio::time::{interval, Duration};
use slint::SharedString;
use twister::dispatch::signal_ingester::{SignalIngester, SignalMetadata, SampleFormat, SignalType};
use twister::dispatch::audio_ingester::AudioIngester;
use twister::ui::enable_acrylic_blur;

slint::include_modules!();

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ui = TotoHudApplet::new()?;

    // Apply Windows Acrylic effect
    #[cfg(target_os = "windows")]
    enable_acrylic_blur(ui.window());

    let ui_handle = ui.as_weak();

    // Instantiate Concrete Ingester for the physics pipeline
    let audio_ingester = AudioIngester::new();

    // The 100Hz BSS Unified Field Tracking Loop
    tokio::spawn(async move {
        let mut tick = interval(Duration::from_millis(10)); // 100Hz
        let mut simulated_time: f32 = 0.0;
        let sample_rate = 192_000;

        let audio_metadata = SignalMetadata {
            signal_type: SignalType::Audio,
            sample_rate_hz: sample_rate,
            carrier_freq_hz: None,
            num_channels: 1,
            sample_format: SampleFormat::I16,
        };

        loop {
            tick.tick().await;
            simulated_time += 0.05;

            // Generate mock hardware bytes covering multiple spectrums to feed the ingester
            let num_samples = 512;
            let mut mock_pcm = Vec::with_capacity(num_samples * 2);
            for i in 0..num_samples {
                // Emulate 60Hz hum, 10.625kHz, and 85kHz folding
                let t = i as f32 / sample_rate as f32 + simulated_time;
                let hz_60 = (t * 60.0 * std::f32::consts::TAU).sin() * 0.3;
                let hz_10k = (t * 10625.0 * std::f32::consts::TAU).sin() * 0.4;
                let hz_85k = (t * 85000.0 * std::f32::consts::TAU).sin() * 0.3; // Folded into Cyan
                let sample_f32 = hz_60 + hz_10k + hz_85k;
                let sample_i16 = (sample_f32 * 32767.0) as i16;
                mock_pcm.extend_from_slice(&sample_i16.to_le_bytes());
            }

            // Ingest to unified particles! (The Brawn)
            let particles = audio_ingester.ingest(&mock_pcm, 0, &audio_metadata);

            let canvas_width = 800.0;
            let canvas_height = 350.0;
            let center_y = canvas_height / 2.0;

            let mut paths = vec![
                String::with_capacity(num_samples * 20); 12
            ];
            for path in paths.iter_mut() {
                path.push_str(&format!("M 0 {:.1}", center_y));
            }

            // BSS Unmixing Visualization logic
            // Parse particles and simulate unmixing based on RMS energy & motifs
            let mut total_energy = 0.0;
            for (idx, particle) in particles.iter().take(num_samples).enumerate() {
                total_energy += particle.energy;

                let x = (idx as f32 / num_samples as f32) * canvas_width;

                // Unmix mapping:
                // Red (Hue 0): 60Hz Baseline (Low frequency, large phase swing)
                let y_red = center_y - (particle.phase_i * 80.0 * (1.0 - particle.energy.min(1.0)));
                paths[0].push_str(&format!(" L {:.1} {:.1}", x, y_red));

                // Cyan (Hue 6): 10.625kHz and 85kHz (Folded octaves)
                let y_cyan = center_y - (particle.phase_q * 140.0 * particle.energy.max(0.5));
                paths[6].push_str(&format!(" L {:.1} {:.1}", x, y_cyan));

                // Violet (Hue 10): High frequency scatter / Noise floor
                let y_violet = center_y - ((particle.phase_i * particle.phase_q) * 60.0) + (rand::random::<f32>() * 10.0 - 5.0);
                paths[10].push_str(&format!(" L {:.1} {:.1}", x, y_violet));

                // Flatline others
                for (path_idx, path) in paths.iter_mut().enumerate() {
                    if path_idx != 0 && path_idx != 6 && path_idx != 10 {
                        let y_flat = center_y + (rand::random::<f32>() * 2.0 - 1.0);
                        path.push_str(&format!(" L {:.1} {:.1}", x, y_flat));
                    }
                }
            }

            let avg_energy = if particles.is_empty() { 0.0 } else { total_energy / particles.len() as f32 };

            let ui_clone = ui_handle.clone();
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(ui) = ui_clone.upgrade() {
                    let engine = ui.global::<TotoEngine>();

                    if engine.get_always_learning() {
                        engine.set_anomaly_score(avg_energy.min(1.0));
                        engine.set_drive(0.250 + avg_energy * 0.1);
                        engine.set_fold(0.700 - avg_energy * 0.05);
                        engine.set_asym(0.150 + (rand::random::<f32>() * 0.1));

                        engine.set_telemetry_text(SharedString::from(format!(
                            "Unmixed Motifs: 60Hz (Red), 10kHz+85kHz Folded (Cyan) | BSS Energy: {:.3}", avg_energy
                        )));

                        engine.set_path_c(SharedString::from(&paths[0]));
                        engine.set_path_cs(SharedString::from(&paths[1]));
                        engine.set_path_d(SharedString::from(&paths[2]));
                        engine.set_path_ds(SharedString::from(&paths[3]));
                        engine.set_path_e(SharedString::from(&paths[4]));
                        engine.set_path_f(SharedString::from(&paths[5]));
                        engine.set_path_fs(SharedString::from(&paths[6]));
                        engine.set_path_g(SharedString::from(&paths[7]));
                        engine.set_path_gs(SharedString::from(&paths[8]));
                        engine.set_path_a(SharedString::from(&paths[9]));
                        engine.set_path_as(SharedString::from(&paths[10]));
                        engine.set_path_b(SharedString::from(&paths[11]));
                    }
                }
            });
        }
    });

    ui.run()?;
    Ok(())
}
