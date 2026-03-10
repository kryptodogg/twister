// examples/waveshaping_mamba_widget.rs
// Clean waveshaping widget example
// Run with: cargo run --example waveshaping_mamba_widget

slint::slint! {
    import { Button, Slider, ComboBox, VerticalBox, HorizontalBox, GroupBox } from "std-widgets.slint";

    export component AppWindow inherits Window {
        width: 500px;
        height: 450px;
        title: "Auto-Waveshaping Widget";

        in-out property <float> drive: 0.5;
        in-out property <int> mode: 0;
        in-out property <float> tone: 0.5;
        in-out property <string> status: "Ready";

        callback auto_waveshaping();

        background: #1a1a2e;

        VerticalBox {
            padding: 20px;
            spacing: 15px;

            Text {
                text: "🎛️ Auto-Waveshaping";
                font-size: 16px;
                color: #00ff88;
            }

            Button {
                text: "Auto-Waveshape (Mamba)";
                clicked => {
                    root.auto_waveshaping();
                }
            }

            Text {
                text: "Status: " + root.status;
                font-size: 12px;
                color: #ffaa00;
            }

            GroupBox {
                title: "Drive";
                VerticalBox {
                    spacing: 8px;
                    Slider {
                        value <=> root.drive;
                        minimum: 0.0;
                        maximum: 1.0;
                    }
                    Text {
                        text: round(root.drive * 100.0) + "%";
                        color: #00ff88;
                    }
                }
            }

            GroupBox {
                title: "Mode";
                ComboBox {
                    model: ["Sine", "Triangle", "Square"];
                    current-index <=> root.mode;
                }
            }

            GroupBox {
                title: "Tone";
                VerticalBox {
                    spacing: 8px;
                    Slider {
                        value <=> root.tone;
                        minimum: 0.0;
                        maximum: 1.0;
                    }
                    Text {
                        text: round(root.tone * 100.0) + "%";
                        color: #00ff88;
                    }
                }
            }
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ui = AppWindow::new()?;
    let ui_weak = ui.as_weak();

    ui.on_auto_waveshaping(move || {
        println!("🤖 Auto-waveshaping activated!");

        // Simulate Mamba prediction
        let predicted_drive = (rand::random::<f32>() * 0.5) + 0.3;
        let predicted_mode = (rand::random::<u32>() % 3) as i32;
        let predicted_tone = rand::random::<f32>();

        println!("  Drive:  {:.2}", predicted_drive);
        println!("  Mode:   {}", predicted_mode);
        println!("  Tone:   {:.2}", predicted_tone);

        // Update UI
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_drive(predicted_drive);
            ui.set_mode(predicted_mode);
            ui.set_tone(predicted_tone);
            ui.set_status("✓ Mamba prediction applied".into());
        }
    });

    ui.run()?;
    Ok(())
}
