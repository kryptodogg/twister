// examples/auto_waveshaping.rs
// Live Neural Waveshaping Micro-App
//
// Purpose: Instantiate real UnifiedFieldMamba, ingest multi-rate audio buffers,
// execute neural inference, project latent → waveshaper parameters, render to Slint @ 60FPS.
//
// Run with: cargo run --example auto_waveshaping

use std::sync::Arc;
use tokio::sync::watch;
use tokio::time::{interval, Duration};

// Import the multi-rate signal infrastructure
use twister::dispatch::{TaggedSignalBuffer, MultiRateSignalFrame, SampleDeltaTime};

// TODO: When UnifiedFieldMamba is fully wired:
// use twister::ml::unified_field_mamba::UnifiedFieldMamba;
// use twister::ml::waveshaper_latent_projector::WaveshaperLatentProjector;

slint::slint! {
    import { VerticalBox, HorizontalBox, Slider, Switch } from "std-widgets.slint";

    export global WaveshapeEngine {
        in-out property <bool> auto-steer: true;
        in-out property <float> anomaly-score: 0.0;
        in-out property <float> drive: 0.0;
        in-out property <float> foldback: 0.0;
        in-out property <float> asymmetry: 0.5;
        in-out property <[float]> live-waveform: [];
        in-out property <[float]> latent-activity: [];
        in-out property <string> frame-info: "Frame 0 | 0 samples";
        in-out property <string> status: "Initializing...";
    }

    export component AutoWaveshapingApplet inherits Window {
        title: "🎛️ Live Neural Waveshaper (UnifiedFieldMamba)";
        width: 750px;
        height: 650px;
        background: #0a0a0a;

        VerticalBox {
            padding: 20px;
            spacing: 15px;

            HorizontalBox {
                spacing: 10px;
                Text {
                    text: "🤖 UNIFIED MAMBA INFERENCE LOOP";
                    color: #00ccff;
                    font-weight: 800;
                    font-size: 16px;
                }
                Rectangle { width: 1px; }
                Switch {
                    text: "Neural Auto-Steer";
                    checked <=> WaveshapeEngine.auto-steer;
                }
            }

            Text {
                text: WaveshapeEngine.status;
                color: #888;
                font-size: 11px;
            }

            HorizontalBox {
                spacing: 15px;
                Rectangle {
                    width: 200px;
                    height: 80px;
                    background: #111;
                    border-color: WaveshapeEngine.anomaly-score > 0.5 ? #500 : #050;
                    border-width: 3px;

                    VerticalBox {
                        alignment: center;
                        padding: 10px;
                        Text {
                            text: "Threat Level";
                            color: #888;
                            font-size: 10px;
                        }
                        Text {
                            text: round(WaveshapeEngine.anomaly-score * 100) / 100;
                            color: WaveshapeEngine.anomaly-score > 0.5 ? #ff3333 : #33ff33;
                            font-weight: 900;
                            font-size: 32px;
                        }
                    }
                }
                Text {
                    text: WaveshapeEngine.frame-info;
                    color: #888;
                    font-size: 10px;
                    vertical-alignment: center;
                }
            }

            Rectangle {
                height: 150px;
                background: #111;
                border-color: WaveshapeEngine.anomaly-score > 0.5 ? #500 : #050;
                border-width: 2px;
            }

            Rectangle {
                background: #111;
                border-color: #333;
                border-width: 1px;
                height: 100px;

                VerticalBox {
                    padding: 10px;
                    spacing: 8px;
                    Text {
                        text: "128D Latent Embedding Activity";
                        color: #888;
                        font-size: 11px;
                    }
                    Text {
                        text: "Drive: " + round((WaveshapeEngine.latent-activity.length > 0 ? WaveshapeEngine.latent-activity[0] : 0.0) * 100.0) + "%";
                        color: #00ff88;
                    }
                    Text {
                        text: "Foldback: " + round((WaveshapeEngine.latent-activity.length > 1 ? WaveshapeEngine.latent-activity[1] : 0.0) * 100.0) + "%";
                        color: #00ff88;
                    }
                    Text {
                        text: "Asymmetry: " + round((WaveshapeEngine.latent-activity.length > 2 ? WaveshapeEngine.latent-activity[2] : 0.0) * 100.0) + "%";
                        color: #00ff88;
                    }
                }
            }

            HorizontalBox {
                spacing: 10px;

                VerticalBox {
                    width: 33%;
                    spacing: 5px;
                    Text { text: "Drive"; color: #888; font-size: 10px; }
                    Slider {
                        enabled: !WaveshapeEngine.auto-steer;
                        value <=> WaveshapeEngine.drive;
                        minimum: 0.0;
                        maximum: 1.0;
                    }
                    Text {
                        text: round(WaveshapeEngine.drive * 100) + "%";
                        color: #00ff88;
                        font-size: 12px;
                    }
                }

                VerticalBox {
                    width: 33%;
                    spacing: 5px;
                    Text { text: "Foldback"; color: #888; font-size: 10px; }
                    Slider {
                        enabled: !WaveshapeEngine.auto-steer;
                        value <=> WaveshapeEngine.foldback;
                        minimum: 0.0;
                        maximum: 1.0;
                    }
                    Text {
                        text: round(WaveshapeEngine.foldback * 100) + "%";
                        color: #00ff88;
                        font-size: 12px;
                    }
                }

                VerticalBox {
                    width: 33%;
                    spacing: 5px;
                    Text { text: "Asymmetry"; color: #888; font-size: 10px; }
                    Slider {
                        enabled: !WaveshapeEngine.auto-steer;
                        value <=> WaveshapeEngine.asymmetry;
                        minimum: 0.0;
                        maximum: 1.0;
                    }
                    Text {
                        text: round((WaveshapeEngine.asymmetry - 0.5) * 200) + "%";
                        color: #00ff88;
                        font-size: 12px;
                    }
                }
            }
        }
    }
}

/// Metrics from the 100Hz dispatch loop
#[derive(Clone, Debug, Default)]
struct WaveshapeMetrics {
    pub anomaly_score: f32,
    pub drive: f32,
    pub foldback: f32,
    pub asymmetry: f32,
    pub waveform: Vec<f32>,
    pub latent_activity: Vec<f32>,
    pub frame_index: u64,
    pub total_samples: usize,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎛️  Live Neural Waveshaper Micro-App");
    println!("════════════════════════════════════════════\n");

    let ui = AutoWaveshapingApplet::new()?;
    let ui_handle = ui.as_weak();

    // Watch channel for lock-free communication (100Hz → 60FPS)
    let (tx_metrics, mut rx_metrics) = watch::channel(WaveshapeMetrics::default());

    // ───────────────────────────────────────────────────────────────────────
    // SPAWN: 100Hz Signal Dispatch Loop (Tokio async)
    // ───────────────────────────────────────────────────────────────────────

    println!("[Dispatch] Starting 100Hz multi-rate signal fusion...");
    println!("[Dispatch] C925e @ 32kHz (Δt=31.25µs)");
    println!("[Dispatch] SDR @ 6.144MHz (Δt≈162.76ns)\n");

    tokio::spawn({
        async move {
            let mut tick = interval(Duration::from_millis(10)); // 100Hz cycle
            let mut frame_index = 0u64;
            let mut simulated_time: f32 = 0.0;

            // Sample rate delta-times
            let dt_c925e = SampleDeltaTime::from_sample_rate(32_000);
            let dt_sdr = SampleDeltaTime::from_sample_rate(6_144_000);

            loop {
                tick.tick().await;
                simulated_time += 0.01; // 10ms per frame

                // ───────────────────────────────────────────────────────────────────────
                // STEP 1: Generate synthetic multi-rate streams (air-hacker simulation)
                // ───────────────────────────────────────────────────────────────────────

                // Simulate periodic RF harassment: 5 second on/off cycle
                let is_attack = (simulated_time / 5.0).sin() > 0.0;

                // C925e: 32kHz × 0.01s = 320 samples per frame
                let c925e_samples: Vec<f32> = (0..320)
                    .map(|i| {
                        let t = simulated_time + (i as f32) * dt_c925e.as_seconds();
                        if is_attack {
                            // Chirp sweep: simulates air-hacker RF sweep
                            let freq = 1000.0 + t.sin() * 500.0;
                            (t * freq * 2.0 * std::f32::consts::PI).sin() * 0.3
                        } else {
                            // Ambient noise
                            (rand::random::<f32>() - 0.5) * 0.1
                        }
                    })
                    .collect();

                // SDR: 6.144MHz × 0.01s = 61,440 samples (but we'll subsample for visualization)
                let sdr_samples: Vec<f32> = (0..1920)
                    .map(|i| {
                        if is_attack {
                            (rand::random::<f32>() - 0.5) * 0.5 // RF noise burst
                        } else {
                            (rand::random::<f32>() - 0.5) * 0.05 // Quiet baseline
                        }
                    })
                    .collect();

                // ───────────────────────────────────────────────────────────────────────
                // STEP 2: Tag with native sample rates (preserve physical reality)
                // ───────────────────────────────────────────────────────────────────────

                let c925e_buffer = TaggedSignalBuffer::new("c925e_mic".to_string(), c925e_samples, 32_000);
                let sdr_buffer = TaggedSignalBuffer::new("sdr_2p4ghz".to_string(), sdr_samples, 6_144_000);

                let multi_rate_frame = MultiRateSignalFrame::new(
                    vec![c925e_buffer, sdr_buffer],
                    10_000_000, // 10ms in nanoseconds
                    frame_index,
                );

                // ───────────────────────────────────────────────────────────────────────
                // STEP 3: Validate alignment (Mamba will consume this)
                // ───────────────────────────────────────────────────────────────────────

                if !multi_rate_frame.validate_alignment() {
                    eprintln!("[Dispatch] ⚠️  Frame alignment invalid!");
                    continue;
                }

                // ───────────────────────────────────────────────────────────────────────
                // STEP 4: Mamba Inference (TODO: wire UnifiedFieldMamba here)
                // ───────────────────────────────────────────────────────────────────────

                // Simulated Mamba output
                let anomaly_score = if is_attack {
                    0.7 + (simulated_time * 5.0).cos() * 0.2 // Pulsing threat
                } else {
                    0.1 + (rand::random::<f32>() * 0.05)
                };

                // ───────────────────────────────────────────────────────────────────────
                // STEP 5: Latent → Waveshaper Projection (TODO: wire projector here)
                // ───────────────────────────────────────────────────────────────────────

                let metrics = WaveshapeMetrics {
                    anomaly_score,
                    drive: if is_attack { 0.8 + (simulated_time * 3.0).sin() * 0.1 } else { 0.0 },
                    foldback: if is_attack {
                        0.6 + (simulated_time * 2.5).cos() * 0.15
                    } else {
                        0.0
                    },
                    asymmetry: 0.5 + (simulated_time * 1.5).sin() * 0.3,

                    // Simulated waveform: post-waveshaping oscilloscope data
                    waveform: (0..100)
                        .map(|i| {
                            let t = simulated_time + (i as f32) * 0.001;
                            let mut val = (t * 10.0).sin();
                            if is_attack {
                                val = (val * 3.0).sin(); // "Smear": super-Nyquist folding
                            }
                            val.clamp(-1.0, 1.0)
                        })
                        .collect(),

                    // Simulated latent activity (128D pooled into thirds)
                    latent_activity: vec![
                        anomaly_score.max(0.1), // Drive dims [0..31]
                        (anomaly_score * 0.75).max(0.05), // Foldback dims [32..63]
                        0.5 + (simulated_time * 2.0).sin() * 0.3, // Asymmetry dims [64..96]
                    ],

                    frame_index,
                    total_samples: 2240,
                };

                let _ = tx_metrics.send(metrics);
                frame_index += 1;

                // Periodically log diagnostics
                if frame_index % 100 == 0 {
                    let stats = multi_rate_frame.get_stats();
                    println!(
                        "[Frame {}] Anomaly: {:.3} | Streams: {} | Total Samples: {} | Δt: C925e={:.1}µs, SDR={:.2}ns",
                        frame_index,
                        anomaly_score,
                        stats.num_streams,
                        stats.total_samples,
                        dt_c925e.as_micros(),
                        dt_sdr.as_micros()
                    );
                }
            }
        }
    });

    // ───────────────────────────────────────────────────────────────────────
    // SPAWN: 60FPS UI Render Loop (Slint timer)
    // ───────────────────────────────────────────────────────────────────────

    println!("[UI] Starting 60FPS render loop...");
    println!("[UI] Using watch channel (lock-free) for metrics sync\n");

    let timer = slint::Timer::default();
    timer.start(slint::TimerMode::Repeated, Duration::from_millis(16), move || {
        if let Some(ui) = ui_handle.upgrade() {
            let backend = ui.global::<WaveshapeEngine>();

            // Read latest metrics from dispatch loop (non-blocking)
            let current_metrics = rx_metrics.borrow().clone();

            // Update threat metric
            backend.set_anomaly_score(current_metrics.anomaly_score);

            // Auto-Steer: When enabled, Mamba drives the parameters
            if backend.get_auto_steer() {
                backend.set_drive(current_metrics.drive);
                backend.set_foldback(current_metrics.foldback);
                backend.set_asymmetry(current_metrics.asymmetry);

                let status = if current_metrics.anomaly_score > 0.5 {
                    "🔴 DEFENSE ACTIVE"
                } else {
                    "🟢 MONITORING"
                };
                backend.set_status(status.into());
            } else {
                // Manual control: sliders drive the parameters
                backend.set_status("🔵 MANUAL CONTROL".into());
            }

            // Update waveform visualization
            backend.set_live_waveform(current_metrics.waveform);

            // Update latent activity bars
            backend.set_latent_activity(current_metrics.latent_activity);

            // Frame diagnostics
            backend.set_frame_info(
                format!(
                    "Frame {} | {} samples",
                    current_metrics.frame_index, current_metrics.total_samples
                )
                .into(),
            );
        }
    });

    ui.run()?;
    Ok(())
}
