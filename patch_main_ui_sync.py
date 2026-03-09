import re

with open("src/main.rs", "r") as f:
    content = f.read()

if "ui.set_gate_status" not in content:
    sync_code = """                ui.set_rtl_scanning(st.get_sdr_sweeping());
                ui.set_training_pairs(training_session_timer.total_pairs() as i32);

                if let Ok(gs) = st.gate_status.try_lock() {
                    ui.set_gate_status(gs.clone().into());
                }
                if let Ok(gr) = st.last_gate_reason.try_lock() {
                    ui.set_last_gate_reason(gr.clone().into());
                }
                ui.set_training_pairs_dropped(st.training_pairs_dropped.load(Ordering::Relaxed) as i32);
                ui.set_gate_rejections_low_anomaly(st.gate_rejections_low_anomaly.load(Ordering::Relaxed) as i32);
                ui.set_gate_rejections_low_confidence(st.gate_rejections_low_confidence.load(Ordering::Relaxed) as i32);"""

    content = content.replace("                ui.set_rtl_scanning(st.get_sdr_sweeping());\n                ui.set_training_pairs(training_session_timer.total_pairs() as i32);", sync_code)

with open("src/main.rs", "w") as f:
    f.write(content)
