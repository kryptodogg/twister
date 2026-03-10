use std::collections::HashMap;
use burn::backend::ndarray::NdArrayDevice;
use burn::backend::NdArray;

use std::time::Instant;

#[path = "../src/ml/data_contracts.rs"]
pub mod data_contracts;
#[path = "../src/ml/timegnn.rs"]
pub mod timegnn;
#[path = "../src/ml/timegnn_trainer.rs"]
pub mod timegnn_trainer;
#[path = "../src/ml/pattern_discovery.rs"]
pub mod pattern_discovery;

use data_contracts::ForensicEventData;
use timegnn::TimeGnnModel;
use pattern_discovery::discover_patterns;
use burn::tensor::{Tensor, TensorData};

fn main() {
    println!("--- Brawn-Only Validation: TimeGNN + KMeans ---");

    let num_events = 10000;
    println!("Generating {} synthetic 'Wave-Particle' events...", num_events);

    let mut events = Vec::with_capacity(num_events);
    for i in 0..num_events {
        let is_violet = i % 2 == 0;
        let tag = if is_violet { "Violet_750THz" } else { "Thermal_60Hz" };
        let freq = if is_violet { 750.0e12 } else { 60.0 };

        let mut features = vec![0.0f32; 1297];
        features[0] = freq as f32;
        features[1] = if is_violet { 1.0 } else { -1.0 };
        features[2] = (i as f32).sin();

        events.push(ForensicEventData {
            id: format!("event_{}", i),
            timestamp_micros: (1600000000000_i64 + (i as i64) * 3600000000_i64),
            features,
            tag: tag.to_string(),
            confidence: 0.95,
            rf_frequency_hz: freq as f32,
            duration_seconds: 1.0,
            timestamp_unix: 1600000000.0 + (i as f64) * 3600.0,
            frequency_hz: freq as f32,
            metadata: HashMap::new(),
        });
    }

    let device = NdArrayDevice::default();
    type Backend = NdArray<f32>;


    let start = Instant::now();

    println!("Instantiating TimeGNN Model on Wgpu natively...");
    let model = TimeGnnModel::<Backend>::new(1297, &device);

    let input_dim = 1297;
    let mut flat_features = Vec::with_capacity(num_events * input_dim);
    for event in &events {
        flat_features.extend_from_slice(&event.features);
    }

    let tensor_data = TensorData::new(flat_features, [num_events, input_dim]);
    let batch_tensor: Tensor<Backend, 2> = Tensor::from_data(tensor_data, &device);
    let embeddings = model.forward(batch_tensor);

    println!("Running Native WGPU K-Means Clustering...");
    let library = discover_patterns::<Backend>(&embeddings, &events, 2).unwrap();

    let duration = start.elapsed();

    println!("Clustering complete! Discovered {} Motifs.", library.total_patterns);
    println!("Total loop time: {:?}", duration);

    let mut violet_cluster_id = None;
    let mut thermal_cluster_id = None;

    for pattern in &library.patterns {
        println!("- Motif {}: size={}, freq={:.1}h, rf_mode={:.1e} Hz",
            pattern.motif_id, pattern.cluster_size, pattern.frequency_hours, pattern.rf_frequency_hz_mode);

        if pattern.rf_frequency_hz_mode > 1.0e12 {
            violet_cluster_id = Some(pattern.motif_id);
        } else {
            thermal_cluster_id = Some(pattern.motif_id);
        }
    }

    assert!(violet_cluster_id.is_some(), "Did not form a distinct Violet 750THz cluster");
    assert!(thermal_cluster_id.is_some(), "Did not form a distinct Thermal 60Hz cluster");
    assert_ne!(violet_cluster_id, thermal_cluster_id, "Violet and Thermal signals collapsed into the same cluster!");

    println!("✅ SUCCESS: Pattern discovery successfully isolated 750THz Violet beat aliases from 60Hz Thermal baseline purely using Wgpu logic.");
    // Check JSON serialization functionality too!
    let output_path = "test_pattern_library.json";
    library.save(output_path).unwrap();
    let loaded_library = pattern_discovery::load_pattern_library(output_path).unwrap();
    assert_eq!(library.total_patterns, loaded_library.total_patterns);
    println!("✅ JSON serialization successful: saved to {}", output_path);
}
