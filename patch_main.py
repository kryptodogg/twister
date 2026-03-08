import re

with open("src/main.rs", "r") as f:
    content = f.read()

# Find the bounded channels creation around line 114
channel_def = """    let (merge_tx, merge_rx) = crossbeam_channel::bounded::<Vec<f32>>(256);
    let (tdoa_tx, tdoa_rx) = tdoa_channel();"""
new_channel_def = """    let (merge_tx, merge_rx) = crossbeam_channel::bounded::<Vec<f32>>(256);
    let (feature_tx, feature_rx) = crossbeam_channel::bounded::<(crate::ml::modular_features::SignalFeaturePayload, burn::tensor::Tensor<burn::backend::NdArray, 1>)>(256);
    let (tdoa_tx, tdoa_rx) = tdoa_channel();"""
content = content.replace(channel_def, new_channel_def)

# Actually let's just find `let (merge_tx, merge_rx) = bounded::<Vec<f32>>(256);` and replace
if "let (merge_tx, merge_rx) = bounded::<Vec<f32>>(256);" in content:
    content = content.replace("let (merge_tx, merge_rx) = bounded::<Vec<f32>>(256);", "let (merge_tx, merge_rx) = crossbeam_channel::bounded::<Vec<f32>>(256);\n    let (feature_tx, feature_rx) = crossbeam_channel::bounded::<(crate::ml::modular_features::SignalFeaturePayload, burn::tensor::Tensor<burn::backend::NdArray, 1>)>(256);")


# Find the dispatch loop spawn
dispatch_spawn = """    let gpu_shared_disp = gpu_shared.clone();
    let session_identity_clone = session_identity.clone();

    tokio::spawn(async move {"""
new_dispatch_spawn = """    let gpu_shared_disp = gpu_shared.clone();
    let session_identity_clone = session_identity.clone();
    let feature_tx = feature_tx.clone();

    tokio::spawn(async move {"""
content = content.replace(dispatch_spawn, new_dispatch_spawn)


# Find the chunk processing in the dispatch loop
chunk_processing = """            let (filtered_chunk, pdm_spike_count) = audio::reject_pdm_spikes(&chunk);
            chunk = filtered_chunk;
            if pdm_spike_count > 0 {"""
new_chunk_processing = """            let (filtered_chunk, pdm_spike_count) = audio::reject_pdm_spikes(&chunk);
            chunk = filtered_chunk;

            // Extract modular features based on flags
            let feature_flags = state_disp.get_feature_flags();
            let payload = crate::ml::modular_features::SignalFeaturePayload {
                audio_samples: chunk.clone(),
                freq_hz: state_disp.get_detected_freq(),
                tdoa_confidence: Some(state_disp.get_beam_confidence()),
                device_corr: None,
                vbuffer_coherence: None,
                anc_phase: None,
                harmonic_energy: None,
            };
            let device = Default::default();
            let extractor = crate::ml::modular_features::ModularFeatureExtractor::<burn::backend::ndarray::NdArray<f32>>::new(&device);
            let (feature_vec, _) = extractor.extract(&payload, &feature_flags);
            let _ = feature_tx.try_send((payload, feature_vec));

            if pdm_spike_count > 0 {"""
content = content.replace(chunk_processing, new_chunk_processing)

with open("src/main.rs", "w") as f:
    f.write(content)
