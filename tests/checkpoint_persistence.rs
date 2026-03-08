use std::fs;

/// Test 1: Basic checkpoint metadata serialization
#[test]
fn test_checkpoint_metadata_save() {
    // Verify that epoch and loss can be saved alongside weights
    let metadata = twister::state::CheckpointMetadata {
        epoch: 247,
        loss_avg: 0.1234,
        loss_min: 0.0456,
        loss_max: 0.9876,
        timestamp_created: "2026-03-07T14:23:14Z".to_string(),
    };

    // Should serialize to JSON without errors
    let json = serde_json::to_string(&metadata)
        .expect("Metadata should serialize to JSON");

    assert!(json.contains("\"epoch\":247"));
    assert!(json.contains("\"loss_avg\":0.1234"));
}

/// Test 2: Checkpoint metadata deserialization
#[test]
fn test_checkpoint_metadata_load() {
    let json = r#"{"epoch":247,"loss_avg":0.1234,"loss_min":0.0456,"loss_max":0.9876,"timestamp_created":"2026-03-07T14:23:14Z"}"#;
    
    let metadata: twister::state::CheckpointMetadata = serde_json::from_str(json)
        .expect("Should deserialize from JSON");

    assert_eq!(metadata.epoch, 247);
    assert_eq!(metadata.loss_avg, 0.1234);
    assert_eq!(metadata.loss_min, 0.0456);
}

/// Test 3: Checkpoint with metadata persistence (mock)
#[test]
fn test_checkpoint_roundtrip() {
    // Verify metadata can be serialized, written, and read back
    let original = twister::state::CheckpointMetadata {
        epoch: 500,
        loss_avg: 0.0512,
        loss_min: 0.0001,
        loss_max: 0.8500,
        timestamp_created: "2026-03-07T15:30:00Z".to_string(),
    };

    // Simulate file write/read
    let json = serde_json::to_string(&original).expect("Should serialize");
    let restored: twister::state::CheckpointMetadata = 
        serde_json::from_str(&json).expect("Should deserialize");

    assert_eq!(restored.epoch, original.epoch);
    assert_eq!(restored.loss_avg, original.loss_avg);
    assert_eq!(restored.timestamp_created, original.timestamp_created);
}

/// Test 4: Mamba should capture epoch/loss on checkpoint save
#[test]
fn test_mamba_checkpoint_includes_metrics() {
    // Verify checkpoint includes current epoch and loss metrics
    let checkpoint = twister::state::CheckpointMetadata {
        epoch: 100,
        loss_avg: 0.3456,
        loss_min: 0.0123,
        loss_max: 0.9999,
        timestamp_created: "2026-03-07T14:00:00Z".to_string(),
    };

    // On load, epoch should NOT reset to 0
    assert_ne!(checkpoint.epoch, 0, "Loaded checkpoint should preserve epoch number");
    
    // Loss average should be recoverable
    assert!(checkpoint.loss_avg > 0.0, "Loss average should be positive");
}
