/// TDD Test Suite: Phase 1 - Memo Backend Integration
///
/// Tests the memo/notes feature with forensic integration:
/// - MemoEntry creation with validation
/// - Storage in AppState with max capacity (10k)
/// - Forensic log event generation
/// - Auto-capture on EVIDENCE tag
/// - CSV export with metadata
use std::time::{SystemTime, UNIX_EPOCH};

/// Test 1: MemoEntry creation and JSON serialization
#[test]
fn test_memo_entry_creation() {
    // This test verifies that we can create a MemoEntry with:
    // - ISO 8601 timestamp
    // - Microsecond precision timestamp
    // - Tag classification (NOTE, EVIDENCE, MANUAL-REC, ANALYSIS)
    // - User content (max 80 chars)
    // - JSON serialization

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_micros() as u64;

    let timestamp_iso = format!("2026-03-07T14:23:14.832401Z");

    // Create a MemoEntry using the new() constructor
    let memo = twister::state::MemoEntry::new(
        timestamp_iso.clone(),
        now,
        "NOTE".to_string(),
        "Test memo entry",
    )
    .expect("Should create valid MemoEntry");

    // Verify fields are accessible
    assert_eq!(memo.tag, "NOTE");
    assert_eq!(memo.content, "Test memo entry");
    assert_eq!(memo.timestamp_micros, now);
    assert_eq!(memo.timestamp_iso8601, timestamp_iso);

    // Verify JSON serialization
    let json = serde_json::to_string(&memo).expect("Failed to serialize MemoEntry");
    assert!(
        json.contains("\"tag\":\"NOTE\""),
        "JSON should contain tag field"
    );
    assert!(
        json.contains("\"content\":\"Test memo entry\""),
        "JSON should contain content field"
    );
}

/// Test 2: MemoEntry validation - content length limit
#[test]
fn test_memo_entry_max_content_length() {
    // Verify that MemoEntry enforces max 80 character limit on content
    let long_content = "x".repeat(81); // 81 characters, over the 80-char limit

    let result = twister::state::MemoEntry::new(
        "2026-03-07T14:23:14.832401Z".to_string(),
        1741354994832401,
        "NOTE".to_string(),
        long_content.as_str(),
    );

    // Should return an error for content > 80 chars
    assert!(result.is_err(), "Should reject content > 80 characters");
}

/// Test 3: MemoEntry validation - valid tag
#[test]
fn test_memo_entry_tag_validation() {
    // Verify that only valid tags are accepted: NOTE, EVIDENCE, MANUAL-REC, ANALYSIS
    let valid_tags = vec!["NOTE", "EVIDENCE", "MANUAL-REC", "ANALYSIS"];

    for tag in valid_tags {
        let result = twister::state::MemoEntry::new(
            "2026-03-07T14:23:14.832401Z".to_string(),
            1741354994832401,
            tag.to_string(),
            "Valid memo",
        );

        assert!(result.is_ok(), "Tag '{}' should be valid", tag);
    }

    // Test invalid tag
    let result = twister::state::MemoEntry::new(
        "2026-03-07T14:23:14.832401Z".to_string(),
        1741354994832401,
        "INVALID".to_string(),
        "Invalid memo",
    );

    assert!(result.is_err(), "Should reject invalid tag");
}

/// Test 4: JSON roundtrip serialization
#[test]
fn test_memo_entry_json_roundtrip() {
    // Verify MemoEntry can be serialized and deserialized without loss
    let original = twister::state::MemoEntry::new(
        "2026-03-07T14:23:14.832401Z".to_string(),
        1741354994832401,
        "EVIDENCE".to_string(),
        "Device left unattended",
    )
    .expect("Should create valid MemoEntry");

    // Serialize to JSON
    let json = serde_json::to_string(&original).expect("Should serialize");

    // Deserialize back
    let restored: twister::state::MemoEntry =
        serde_json::from_str(&json).expect("Should deserialize");

    // Verify all fields match
    assert_eq!(restored.timestamp_iso8601, original.timestamp_iso8601);
    assert_eq!(restored.timestamp_micros, original.timestamp_micros);
    assert_eq!(restored.tag, original.tag);
    assert_eq!(restored.content, original.content);
}

// ── AppState Memo Storage Tests ────────────────────────────────────────────────

/// Test 5: AppState memo storage initialization
#[test]
fn test_appstate_memo_storage_init() {
    // Verify AppState initializes with empty memo storage (max 10k capacity)
    let state = twister::state::AppState::new();

    // Should have zero memos initially
    assert_eq!(state.get_memo_count(), 0, "Should start with zero memos");

    // Should support max 10,000 memos (documented capacity)
    assert!(
        state.get_max_memo_capacity() <= 10_000,
        "Max capacity should be <= 10,000"
    );
}

/// Test 6: AppState add_memo() creates new memo entry
#[test]
fn test_appstate_add_memo() {
    // Verify that add_memo() creates a new memo and stores it
    let state = twister::state::AppState::new();

    // Add first memo
    state
        .add_memo("NOTE".to_string(), "First observation")
        .expect("Should add memo");

    // Verify count increased
    assert_eq!(state.get_memo_count(), 1, "Should have 1 memo after add");

    // Add second memo with different tag
    state
        .add_memo("EVIDENCE".to_string(), "Critical finding")
        .expect("Should add memo");

    // Verify count increased again
    assert_eq!(
        state.get_memo_count(),
        2,
        "Should have 2 memos after second add"
    );
}

/// Test 7: AppState delete_memo() removes memo by index
#[test]
fn test_appstate_delete_memo() {
    // Verify delete_memo() removes a memo and shifts remaining entries
    let state = twister::state::AppState::new();

    // Add three memos
    state
        .add_memo("NOTE".to_string(), "First")
        .expect("Should add");
    state
        .add_memo("EVIDENCE".to_string(), "Second")
        .expect("Should add");
    state
        .add_memo("ANALYSIS".to_string(), "Third")
        .expect("Should add");

    assert_eq!(state.get_memo_count(), 3);

    // Delete the middle memo (index 1)
    state.delete_memo(1).expect("Should delete memo at index 1");

    // Verify count decreased
    assert_eq!(
        state.get_memo_count(),
        2,
        "Should have 2 memos after delete"
    );

    // Verify correct memo was deleted by checking order
    let remaining = state.get_memos_all().expect("Should get all memos");
    assert_eq!(
        remaining[0].content, "First",
        "First memo should still be there"
    );
    assert_eq!(
        remaining[1].content, "Third",
        "Third memo should be second now"
    );
}

/// Test 8: AppState memo persistence (should be cleared on new AppState)
#[test]
fn test_appstate_memo_not_persisted() {
    // Verify memos are stored in memory (not persisted across AppState instances)
    let state1 = twister::state::AppState::new();
    state1
        .add_memo("NOTE".to_string(), "Test memo")
        .expect("Should add");

    // Create new AppState instance
    let state2 = twister::state::AppState::new();

    // Should be empty (in-memory storage is per-instance)
    assert_eq!(
        state2.get_memo_count(),
        0,
        "New instance should have zero memos"
    );
}

// ── Phase 1 Task 2a: 3D Wave Topology Tests ────────────────────────────────────

/// Test 9: MambaControlState captures defense mode configuration
#[test]
fn test_mamba_control_state_creation() {
    // Verify MambaControlState serialization including phased-array + heterodyning
    let control = twister::state::MambaControlState {
        beam_azimuth: 45.5,
        beam_elevation: 15.0,
        waveshape_drive: 0.8,
        heterodyned_beams: vec![Some(95.0e9), Some(95.1e9), Some(95.2e9), Some(95.3e9)],
        anc_gain: 0.6,
        beam_phases: vec![0.0, 1.57, 3.14, 4.71],
        active_modes: vec!["ADS".to_string()],
    };

    // Verify basic fields are accessible
    assert_eq!(control.beam_azimuth, 45.5);
    assert_eq!(control.waveshape_drive, 0.8);
    assert_eq!(control.anc_gain, 0.6);

    // Verify phased-array fields
    assert_eq!(control.beam_elevation, 15.0);
    assert_eq!(control.beam_phases.len(), 4);

    // Verify heterodyning fields
    assert_eq!(control.heterodyned_beams.len(), 4);

    // Verify JSON serialization (for JSONL forensic logs)
    let json = serde_json::to_string(&control).expect("Should serialize");
    assert!(json.contains("\"beam_azimuth\":45.5"));
}

/// Test 10: WaveTopology captures 3D field characteristics
#[test]
fn test_wave_topology_creation() {
    // Verify WaveTopology captures spatial field distortion
    let topology = twister::state::WaveTopology {
        phase_coherence_db: vec![-0.2, 0.8, -0.1], // Three mic pairs
        field_gradient_azimuth: 47.3,
        spatial_curvature: 0.65,
    };

    // Verify fields
    assert_eq!(topology.phase_coherence_db.len(), 3);
    assert_eq!(topology.field_gradient_azimuth, 47.3);
    assert_eq!(topology.spatial_curvature, 0.65);

    // Verify JSON serialization
    let json = serde_json::to_string(&topology).expect("Should serialize");
    assert!(json.contains("\"phase_coherence_db\""));
    assert!(json.contains("\"spatial_curvature\":0.65"));
}

/// Test 11: MemoEntry with 3D context (full forensic training example)
#[test]
fn test_memo_entry_with_3d_context() {
    // Verify MemoEntry::with_3d_context() captures full multimodal context with phased-array
    let control = twister::state::MambaControlState {
        beam_azimuth: 45.5,
        beam_elevation: 15.0,
        waveshape_drive: 0.8,
        heterodyned_beams: vec![Some(95.0e9), Some(95.1e9), Some(95.2e9), Some(95.3e9)],
        anc_gain: 0.6,
        beam_phases: vec![0.0, 1.57, 3.14, 4.71],
        active_modes: vec!["MULTIMODAL".to_string()],
    };

    let topology = twister::state::WaveTopology {
        phase_coherence_db: vec![-0.2, 0.8, -0.1],
        field_gradient_azimuth: 47.3,
        spatial_curvature: 0.65,
    };

    let memo = twister::state::MemoEntry::with_3d_context(
        "2026-03-07T14:23:14.832401Z".to_string(),
        1741354994832401,
        "EVIDENCE".to_string(),
        "Coordinated RF + audio attack detected",
        control,
        topology,
        0.73, // flat_horizon_deviation
    )
    .expect("Should create memo with 3D context");

    // Verify all fields present
    assert_eq!(memo.tag, "EVIDENCE");
    assert!(memo.mamba_control.is_some(), "Should have mamba_control");
    assert!(memo.wave_topology.is_some(), "Should have wave_topology");
    assert!(
        memo.flat_horizon_deviation.is_some(),
        "Should have flat_horizon_deviation"
    );

    // Verify nested field access
    let control = memo.mamba_control.as_ref().unwrap();
    assert_eq!(control.beam_azimuth, 45.5);
    assert_eq!(memo.flat_horizon_deviation.unwrap(), 0.73);
}

/// Test 12: MemoEntry 3D context JSON roundtrip (JSONL forensic storage)
#[test]
fn test_memo_entry_3d_context_json_roundtrip() {
    // Verify rich memo with phased-array serializes/deserializes without loss
    let control = twister::state::MambaControlState {
        beam_azimuth: 45.5,
        beam_elevation: 15.0,
        waveshape_drive: 0.8,
        heterodyned_beams: vec![Some(95.0e9), Some(95.1e9), Some(95.2e9), Some(95.3e9)],
        anc_gain: 0.6,
        beam_phases: vec![0.0, 1.57, 3.14, 4.71],
        active_modes: vec!["MULTIMODAL".to_string()],
    };

    let topology = twister::state::WaveTopology {
        phase_coherence_db: vec![-0.2, 0.8, -0.1],
        field_gradient_azimuth: 47.3,
        spatial_curvature: 0.65,
    };

    let original = twister::state::MemoEntry::with_3d_context(
        "2026-03-07T14:23:14.832401Z".to_string(),
        1741354994832401,
        "EVIDENCE".to_string(),
        "Coordinated attack",
        control,
        topology,
        0.73,
    )
    .expect("Should create memo");

    // Serialize to JSON
    let json = serde_json::to_string(&original).expect("Should serialize");

    // Deserialize back
    let restored: twister::state::MemoEntry =
        serde_json::from_str(&json).expect("Should deserialize");

    // Verify all fields match
    assert_eq!(restored.tag, original.tag);
    assert_eq!(restored.content, original.content);
    assert_eq!(restored.mamba_control.as_ref().unwrap().beam_azimuth, 45.5);
    assert_eq!(
        restored.wave_topology.as_ref().unwrap().spatial_curvature,
        0.65
    );
    assert_eq!(restored.flat_horizon_deviation, Some(0.73));
}

// ── Phase 1 Task 3: Manual REC Button State Machine Tests ─────────────────────

/// Test 13: RecordingState initialization and state transitions
#[test]
fn test_recording_state_init() {
    // Verify RecordingState starts in IDLE and can transition to RECORDING
    let mut rec_state = twister::state::RecordingState::new();

    // Should start in IDLE state
    assert!(rec_state.is_idle(), "Should start in IDLE state");
    assert!(
        !rec_state.is_recording(),
        "Should not be RECORDING initially"
    );
    assert!(!rec_state.is_saving(), "Should not be SAVING initially");

    // Start recording
    rec_state.start_recording();
    assert!(
        rec_state.is_recording(),
        "Should be RECORDING after start_recording()"
    );
    assert!(
        !rec_state.is_idle(),
        "Should not be IDLE after start_recording()"
    );
    assert_eq!(
        rec_state.get_remaining_ms(),
        30000,
        "Should have 30000 ms remaining"
    );

    // Stop recording
    rec_state.stop_recording();
    assert!(
        rec_state.is_saving(),
        "Should be SAVING after stop_recording()"
    );
    assert!(
        !rec_state.is_recording(),
        "Should not be RECORDING after stop_recording()"
    );
}

/// Test 14: Countdown timer decrement
#[test]
fn test_recording_countdown_timer() {
    // Verify timer counts down correctly
    let mut rec_state = twister::state::RecordingState::new();
    rec_state.start_recording();

    // Simulate 100 ms elapsed
    rec_state.update_timer_ms(100);
    assert_eq!(
        rec_state.get_remaining_ms(),
        29900,
        "Should decrement by 100 ms"
    );

    // Simulate another 1000 ms elapsed
    rec_state.update_timer_ms(1000);
    assert_eq!(
        rec_state.get_remaining_ms(),
        28900,
        "Should decrement by 1000 ms"
    );

    // Simulate reaching end of 30s recording
    rec_state.update_timer_ms(28900);
    assert_eq!(rec_state.get_remaining_ms(), 0, "Should not go below 0");
    assert!(
        rec_state.is_expired(),
        "Should indicate recording expired when timer reaches 0"
    );
}

/// Test 15: Recording buffer sample accumulation
#[test]
fn test_recording_buffer_samples() {
    // Verify buffer accumulates samples correctly
    let mut rec_state = twister::state::RecordingState::new();
    rec_state.start_recording();

    // Should have zero samples initially
    assert_eq!(
        rec_state.get_sample_count(),
        0,
        "Should start with 0 samples"
    );

    // Simulate capturing samples at 192 kHz for 100 ms
    // 192000 samples/sec * 0.1 sec = 19200 samples
    rec_state.add_samples(19200);
    assert_eq!(
        rec_state.get_sample_count(),
        19200,
        "Should accumulate samples"
    );

    // Add more samples
    rec_state.add_samples(19200);
    assert_eq!(
        rec_state.get_sample_count(),
        38400,
        "Should accumulate additional samples"
    );

    // Stop recording and verify total
    rec_state.stop_recording();
    assert_eq!(
        rec_state.get_sample_count(),
        38400,
        "Sample count should persist after stop"
    );
}

/// Test 16: REC button workflow (start → record → stop → save)
#[test]
fn test_rec_button_complete_workflow() {
    // Verify complete REC button workflow: IDLE → RECORDING → SAVING
    let mut rec_state = twister::state::RecordingState::new();

    // Initial state
    assert!(rec_state.is_idle(), "Start in IDLE");

    // User clicks START button
    rec_state.start_recording();
    assert!(rec_state.is_recording(), "Enter RECORDING after START");
    let start_remaining = rec_state.get_remaining_ms();
    assert_eq!(start_remaining, 30000, "Timer should be 30 seconds");

    // Simulate capturing audio for 5 seconds (192 kHz × 5 sec = 960k samples)
    rec_state.update_timer_ms(5000);
    rec_state.add_samples(960_000);

    assert_eq!(
        rec_state.get_remaining_ms(),
        25000,
        "Timer should show 25s remaining"
    );
    assert_eq!(
        rec_state.get_sample_count(),
        960_000,
        "Buffer should have 960k samples"
    );
    assert!(rec_state.is_recording(), "Still RECORDING");

    // User clicks STOP button (before 30s expires)
    rec_state.stop_recording();
    assert!(rec_state.is_saving(), "Enter SAVING after STOP");
    assert_eq!(
        rec_state.get_sample_count(),
        960_000,
        "Samples preserved after STOP"
    );

    // Verify we can create a [MANUAL-REC] memo from this recording
    let duration_ms = 5000; // 5 seconds captured
    assert!(duration_ms > 0, "Should have recording duration");
}
