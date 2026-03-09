import re

with open("src/main.rs", "r") as f:
    content = f.read()

# Find the Mamba inference block
pattern = r"""                        let gate = crate::ml::anomaly_gate::evaluate_gate\(&frame, anomaly, 2\.0\);
                        if gate\.forward_to_trainer \{
                            // We would enqueue to trainer_tx here in a real setup\.
                            // For now, we just log it if confidence is high\.
                            if gate\.confidence > 0\.8 \{
                                // println!\("\[GATE\] Forwarding: \{\}", gate\.reason\);
                            \}
                        \}"""

replacement = """                        let gate = crate::ml::anomaly_gate::evaluate_gate(&frame, anomaly, 2.0);

                        // Sync to UI
                        if let Ok(mut gs) = state_disp.gate_status.lock() {
                            *gs = if gate.forward_to_trainer { "FORWARD".to_string() } else { "REJECTED".to_string() };
                        }
                        if let Ok(mut gr) = state_disp.last_gate_reason.lock() {
                            *gr = gate.reason.clone();
                        }

                        if gate.forward_to_trainer {
                            // High anomaly, check for training data
                            if state_disp.get_training_recording_enabled() && gate.confidence > 0.8 {
                                let tx_cur = if let Ok(tx) = state_disp.tx_mags.lock() {
                                    let mut t = tx.clone(); t.resize(512, 0.0); t
                                } else { vec![0.0; 512] };

                                let mut rx_cur = if let Ok(sdr_mags) = state_disp.sdr_mags.try_lock() {
                                    let mut r = sdr_mags.clone(); r.resize(512, 0.0); r
                                } else {
                                    let mut r = mags.clone(); r.resize(512, 0.0); r
                                };

                                let pair = mamba::TrainingPair::new(
                                    state_disp.get_sdr_center_hz() as u32,
                                    tx_cur,
                                    rx_cur,
                                );

                                if !training_session_disp.try_enqueue(pair) {
                                    state_disp.training_pairs_dropped.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                }
                            }
                        } else {
                            if gate.reason.contains("Below threshold") {
                                state_disp.gate_rejections_low_anomaly.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            } else if gate.reason.contains("low confidence") {
                                state_disp.gate_rejections_low_confidence.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            } else {
                                state_disp.gate_rejections_other.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            }
                        }

                        // Forensic log the gate decision
                        let fdc2 = forensic_disp.clone();
                        let reason = gate.reason.clone();
                        tokio::spawn(async move {
                            if let Ok(mut f) = fdc2.lock() {
                                let _ = f.log_gate_decision(anomaly, gate.confidence, 2.0, gate.forward_to_trainer, &reason);
                            }
                        });"""

content = re.sub(pattern, replacement, content)

with open("src/main.rs", "w") as f:
    f.write(content)
