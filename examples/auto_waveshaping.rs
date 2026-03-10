slint::include_modules!();
use std::sync::Arc;
use tokio::time::{interval, Duration};
use slint::{Weak, SharedString};
use twister::dispatch::signal_ingester::{SignalIngester, SignalMetadata, SampleFormat, SignalType};
use twister::dispatch::audio_ingester::AudioIngester;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ui = AutoWaveshapingApplet::new()?;
    let ui_handle = ui.as_weak();

    // Instantiate Concrete Ingester
    let audio_ingester = AudioIngester::new();

    // 2. The 100Hz Signal Dispatch Loop
    tokio::spawn(async move {
        let mut tick = interval(Duration::from_millis(10)); // 100Hz
        let mut simulated_time: f32 = 0.0;
        let active_sample_rate = 192_000.0;

        let metadata = SignalMetadata {
            signal_type: SignalType::Audio,
            sample_rate_hz: 192_000,
            carrier_freq_hz: None,
            num_channels: 1,
            sample_format: SampleFormat::I16,
        };

        loop {
            tick.tick().await;
            simulated_time += 0.1;

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

            // Ingest to unified particles!
            let particles = audio_ingester.ingest(&mock_pcm, 0, &metadata);

            let anomaly_score = 0.0; // Placeholder

            // Project directly off the particles instead of rigid parameters
            let mut path_commands = String::with_capacity(512 * 15);
            path_commands.push_str("M 0 60");

            // Extract aggregate statistics for visualizer parameters dynamically
            let mut avg_energy = 0.0;
            let mut max_phase = 0.0;
            for particle in particles.iter() {
                avg_energy += particle.energy;
                if particle.phase_i.abs() > max_phase {
                    max_phase = particle.phase_i.abs();
                }
            }
            if !particles.is_empty() {
                avg_energy /= particles.len() as f32;
            }

            // Param calculation based purely on generalized particle physics
            let drive = avg_energy * 2.0;
            let asymmetry = max_phase;
            let foldback = avg_energy.powf(2.0);

            for (i, particle) in particles.iter().take(512).enumerate() {
                let x = (i as f32 / 512.0) * 600.0;
                let smeared_val = particle.phase_i * (1.0 + drive) * (1.0 + asymmetry);
                let y = 60.0 - (smeared_val * 40.0);
                path_commands.push_str(&format!(" L {:.1} {:.1}", x, y));
            }

            let ui_clone = ui_handle.clone();
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(ui) = ui_clone.upgrade() {
                    let backend = ui.global::<WaveshapeEngine>();
                    backend.set_anomaly_score(anomaly_score);

                    if backend.get_auto_steer() {
                        backend.set_drive(drive);
                        backend.set_foldback(foldback);
                        backend.set_asymmetry(asymmetry);
                    }

                    backend.set_live_waveform_path(SharedString::from(path_commands));
                }
            });
        }
    });

    ui.run()?;
    Ok(())
}
