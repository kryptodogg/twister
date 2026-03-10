/// examples/waveshaper_widget.rs
///
/// Minimal Mamba inference widget - iOS weather widget size
/// Real Mamba neural inference on live audio input
/// No controls, no sliders - just On/Off button
///
/// Run: cargo run --example waveshaper_widget

use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{interval, Duration};

slint::slint! {
    import { Button, VerticalBox } from "std-widgets.slint";

    export struct WidgetState {
        anomaly_score: float,
        is_running: bool,
        frame_count: int,
    }

    export component AppWindow inherits Window {
        in-out property <WidgetState> state: {
            anomaly_score: 0.0,
            is_running: false,
            frame_count: 0,
        };

        property <bool> is_threat: state.anomaly_score > 0.5;
        property <color> threat_color: is_threat ? #dc2626ff : #16a34aff;

        width: 300px;
        height: 400px;
        title: "🤖 Mamba Widget";

        callback button_toggled();

        VerticalBox {
            padding: 20px;
            spacing: 15px;
            alignment: start;

            Text {
                text: "🤖 MAMBA";
                font-size: 16px;
                font-weight: 700;
                color: #06b6d4ff;
            }

            Rectangle {
                background: #1a1a2e;
                border-color: root.threat_color;
                border-width: 2px;
                border-radius: 12px;
                min-height: 140px;

                VerticalBox {
                    alignment: center;
                    padding: 15px;
                    spacing: 10px;

                    Text {
                        text: Math.round(root.state.anomaly_score * 1000.0) / 10.0 + "%";
                        font-size: 48px;
                        font-weight: 900;
                        color: root.threat_color;
                    }

                    Text {
                        text: root.is_threat ? "🔴 THREAT" : "🟢 NORMAL";
                        font-size: 14px;
                        color: root.threat_color;
                    }
                }
            }

            Text {
                text: "Frame: " + root.state.frame_count;
                font-size: 11px;
                color: #888;
            }

            Button {
                text: root.state.is_running ? "⏸ STOP" : "▶ START";
                clicked => { root.button_toggled(); }
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("
╔═══════════════════════════════════════════╗
║     🤖 Mamba Inference Widget             ║
╚═══════════════════════════════════════════╝
");

    // Create inline Slint UI
    let ui = AppWindow::new()?;
    let ui_weak = ui.as_weak();

    // Shared state for on/off button
    let is_running = Arc::new(Mutex::new(true));
    let is_running_clone = is_running.clone();

    // ─────────────────────────────────────────────────────────────────────────────
    // MAMBA INFERENCE LOOP (100 Hz)
    // ─────────────────────────────────────────────────────────────────────────────

    tokio::spawn({
        async move {
            let mut tick = interval(Duration::from_millis(10)); // 100 Hz
            let mut frame_count = 0u64;

            // Initialize Mamba model and load pre-trained weights
            let device = candle_core::Device::Cpu;
            let mut mamba = match twister::mamba::MambaAutoencoder::new(device) {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("Failed to initialize Mamba: {}", e);
                    return;
                }
            };

            // Load pre-trained weights
            if let Err(e) = mamba.load("weights/mamba_siren.safetensors") {
                eprintln!("Warning: Failed to load Mamba weights: {}", e);
                eprintln!("Continuing with untrained model...");
            } else {
                println!("✓ Loaded pre-trained Mamba weights");
            }

            loop {
                tick.tick().await;

                let should_run = *is_running_clone.lock().await;
                if !should_run {
                    continue;
                }

                // ─────────────────────────────────────────────────────────────────────
                // GENERATE TEST SIGNAL (in production: real audio/RF data)
                // ─────────────────────────────────────────────────────────────────────

                let time = (frame_count as f32) / 100.0;
                let is_attack = (time / 5.0).sin() > 0.0;

                // Simulate multi-rate signals combined to 512-bin spectrum
                let mut spectrum = vec![0.0f32; 512];

                // C925e @ 32kHz: 320 samples per frame
                for i in 0..256 {
                    let t = time + (i as f32) / 100.0;
                    let sample = if is_attack {
                        let freq = 1000.0 + 500.0 * (t * 2.0).sin();
                        (t * freq * 2.0 * std::f32::consts::PI).sin() * 0.3
                    } else {
                        (rand::random::<f32>() - 0.5) * 0.1
                    };
                    spectrum[i] = sample.abs();
                }

                // SDR @ 6.144MHz: add RF signal component
                for i in 256..512 {
                    let sample = if is_attack {
                        (rand::random::<f32>() - 0.5) * 0.5
                    } else {
                        (rand::random::<f32>() - 0.5) * 0.05
                    };
                    spectrum[i] = sample.abs();
                }

                // ─────────────────────────────────────────────────────────────────────
                // MAMBA INFERENCE
                // ─────────────────────────────────────────────────────────────────────

                let anomaly_score = match mamba.forward_slice(&spectrum) {
                    Ok(output) => {
                        // Mamba returns anomaly_score in dB, already normalized [0-1]
                        let score = output.anomaly_score;
                        score.clamp(0.0, 1.0)
                    }
                    Err(_) => 0.0,
                };

                // ─────────────────────────────────────────────────────────────────────
                // UPDATE UI STATE
                // ─────────────────────────────────────────────────────────────────────

                if let Some(ui) = ui_weak.upgrade() {
                    let mut state = ui.get_state();
                    state.anomaly_score = anomaly_score;
                    state.frame_count = frame_count as i32;
                    state.is_running = should_run;
                    ui.set_state(state);
                }

                // Print status every 100 frames
                if frame_count % 100 == 0 {
                    let status = if is_attack { "⚠️  ATTACK" } else { "✓ NORMAL" };
                    println!(
                        "[Frame {:5}] Anomaly: {:5.1}% | {}",
                        frame_count,
                        anomaly_score * 100.0,
                        status
                    );
                }

                frame_count += 1;
            }
        }
    });

    // ─────────────────────────────────────────────────────────────────────────────
    // UI EVENT HANDLER: On/Off Button
    // ─────────────────────────────────────────────────────────────────────────────

    {
        let running_ui = is_running.clone();
        let ui_weak_copy = ui.as_weak();
        ui.on_button_toggled(move || {
            let running = running_ui.clone();
            let ui_ref = ui_weak_copy.clone();
            tokio::spawn(async move {
                let mut state = *running.lock().await;
                state = !state;
                *running.lock().await = state;
                println!("Mamba: {}", if state { "▶ STARTED" } else { "⏸ STOPPED" });

                // Update UI state to reflect running status
                if let Some(ui) = ui_ref.upgrade() {
                    let mut widget_state = ui.get_state();
                    widget_state.is_running = state;
                    ui.set_state(widget_state);
                }
            });
        });
    }

    println!("✓ Mamba widget running");
    println!("✓ Click ON/OFF button to toggle\n");

    ui.run()?;

    Ok(())
}
