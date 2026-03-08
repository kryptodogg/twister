import re

with open("src/main.rs", "r") as f:
    content = f.read()

# Add impulse channel
channel_def = """    let (feature_tx, feature_rx) = crossbeam_channel::bounded::<(crate::ml::modular_features::SignalFeaturePayload, burn::tensor::Tensor<burn::backend::NdArray, 1>)>(256);"""
new_channel_def = """    let (feature_tx, feature_rx) = crossbeam_channel::bounded::<(crate::ml::modular_features::SignalFeaturePayload, burn::tensor::Tensor<burn::backend::NdArray, 1>)>(256);
    let (impulse_tx, impulse_rx) = crossbeam_channel::bounded::<crate::ml::modular_features::ImpulseTrainEvent>(256);"""
content = content.replace(channel_def, new_channel_def)

# Add state modifications to AppState
# In src/state.rs instead

# Dispatch loop: add impulse_tx clone
dispatch_spawn = """    let feature_tx = feature_tx.clone();

    tokio::spawn(async move {"""
new_dispatch_spawn = """    let feature_tx = feature_tx.clone();
    let impulse_tx = impulse_tx.clone();

    tokio::spawn(async move {"""
content = content.replace(dispatch_spawn, new_dispatch_spawn)


# Inside dispatch loop logic for chunk processing
chunk_processing = """            // Extract modular features based on flags
            let feature_flags = state_disp.get_feature_flags();"""
new_chunk_processing = """            // Extract modular features based on flags
            let feature_flags = state_disp.get_feature_flags();

            // NEW: Detect impulse trains
            if feature_flags.use_impulse_detection {
                let impulse_times = crate::ml::modular_features::detect_impulse_times(&chunk, 0.8);
                if !impulse_times.is_empty() {
                    let (spacing_hz, jitter, confidence) = crate::ml::modular_features::measure_pulse_train_coherence(&chunk, 192_000);
                    if confidence > 0.5 {
                        let impulse_event = crate::ml::modular_features::ImpulseTrainEvent {
                            timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs_f64(),
                            impulse_times,
                            spacing_hz,
                            jitter,
                            confidence,
                            source_device: 0,
                        };
                        let _ = impulse_tx.try_send(impulse_event);
                    }
                }
            }"""
content = content.replace(chunk_processing, new_chunk_processing)

# Add impulse trainer loop
impulse_trainer = """fn _start_trainer_loop(
    state: std::sync::Arc<std::sync::Mutex<crate::state::AppState>>,"""
new_impulse_trainer = """fn _start_impulse_trainer_loop(
    state: std::sync::Arc<std::sync::Mutex<crate::state::AppState>>,
    impulse_rx: crossbeam_channel::Receiver<crate::ml::modular_features::ImpulseTrainEvent>
) {
    tokio::spawn(async move {
        let impulse_model = crate::ml::modular_features::ImpulsePatternModel::new();
        loop {
            if let Ok(impulse_event) = impulse_rx.recv() {
                let pattern = impulse_model.extract_pattern(&impulse_event);
                let anomaly_score = impulse_model.score_anomaly(&pattern);

                let st = state.lock().unwrap();
                st.impulse_anomaly_score.store(anomaly_score, std::sync::atomic::Ordering::Relaxed);

                if anomaly_score > 0.7 {
                    st.harassment_detected.store(true, std::sync::atomic::Ordering::Relaxed);
                }
            } else {
                break;
            }
        }
    });
}

fn _start_trainer_loop(
    state: std::sync::Arc<std::sync::Mutex<crate::state::AppState>>,"""
content = content.replace(impulse_trainer, new_impulse_trainer)

with open("src/main.rs", "w") as f:
    f.write(content)
