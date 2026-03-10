slint::include_modules!();

use tokio::time::{interval, Duration};
use slint::SharedString;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ui = TotoApplet::new()?;
    let ui_handle = ui.as_weak();

    // The 10ms Track B Dispatch Loop (Unmixed buffer handling)
    tokio::spawn(async move {
        let mut tick = interval(Duration::from_millis(10)); // 100Hz
        let mut simulated_time: f32 = 0.0;

        loop {
            tick.tick().await;
            simulated_time += 0.1;

            let num_points = 512;
            let canvas_width = 800.0;
            let canvas_height = 300.0;
            let center_y = canvas_height / 2.0;

            // Preallocate strings for the 12 spectral paths
            let mut paths = vec![
                String::with_capacity(num_points * 20); 12 // 12 paths for Flutopedia scale
            ];

            // Initialize path starting points
            for (idx, path) in paths.iter_mut().enumerate() {
                path.push_str(&format!("M 0 {:.1}", center_y));
            }

            // Simulate programmatic Blind Signal Separation (BSS)
            // Octave Folding logic:
            // - Hue-C (Deep Red): ~60 Hz powerline interference
            // - Hue-FS (Cyan): 10,625 Hz & 85,000 Hz folded octaves
            // - Hue-AS (Violet): High-energy noise motif

            for i in 0..num_points {
                let x = (i as f32 / num_points as f32) * canvas_width;

                // 1. 60Hz Signal -> Deep Red (Index 0: Hue-C)
                let y_red = center_y - ((simulated_time + i as f32 * 0.05).sin() * 50.0);
                paths[0].push_str(&format!(" L {:.1} {:.1}", x, y_red));

                // 2. 10,625 Hz -> Cyan (Index 6: Hue-FS) - spatial coordinate 1
                let y_cyan1 = center_y - ((simulated_time * 2.0 + i as f32 * 0.15).sin() * 80.0);
                paths[6].push_str(&format!(" L {:.1} {:.1}", x, y_cyan1));

                // 3. 85,000 Hz -> Cyan (Index 6: Hue-FS) - spatial coordinate 2 (shifted phase/energy)
                let y_cyan2 = center_y - ((simulated_time * 3.0 + i as f32 * 0.3).cos() * 120.0);
                // We overwrite Cyan to blend both or just show the separated energy
                // Here, we'll let path[6] represent the sum/interference of the cyan-folded signals
                let combined_cyan = center_y - (((y_cyan1 - center_y) + (y_cyan2 - center_y)) * 0.6);
                // Overwriting the previous push for cyan
                let prev_len = paths[6].rfind(" L ").unwrap_or(paths[6].len());
                paths[6].truncate(prev_len);
                paths[6].push_str(&format!(" L {:.1} {:.1}", x, combined_cyan));

                // 4. Violet noise -> Violet (Index 10: Hue-AS)
                let y_violet = center_y - ((simulated_time * 0.5 + i as f32 * 0.02).sin() * 40.0)
                                        + (rand::random::<f32>() * 10.0 - 5.0);
                paths[10].push_str(&format!(" L {:.1} {:.1}", x, y_violet));

                // Fill other hues with flatline or minor noise to show BSS separation
                for (idx, path) in paths.iter_mut().enumerate() {
                    if idx != 0 && idx != 6 && idx != 10 {
                        let y_flat = center_y + (rand::random::<f32>() * 2.0 - 1.0);
                        path.push_str(&format!(" L {:.1} {:.1}", x, y_flat));
                    }
                }
            }

            let ui_clone = ui_handle.clone();
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(ui) = ui_clone.upgrade() {
                    let engine = ui.global::<TotoEngine>();

                    if engine.get_always_learning() {
                        engine.set_anomaly_score(0.150 + (rand::random::<f32>() * 0.05));
                        engine.set_drive(0.250 + (rand::random::<f32>() * 0.05));
                        engine.set_fold(0.700 + (rand::random::<f32>() * 0.05));
                        engine.set_asym(0.150 + (rand::random::<f32>() * 0.05));

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
