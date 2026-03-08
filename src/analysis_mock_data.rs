/// Mock data sources for ANALYSIS tab visualization
/// Task D.2: Provides test data for temporal scatter, heatmap, dendrogram, and correlation graph

use std::f32;

#[derive(Clone, Debug)]
pub struct AnalysisEvent {
    pub timestamp_weeks: f32,
    pub intensity: f32,
    pub cluster_type: u32, // 0=aggressive, 1=subtle, 2=ongoing, 3=background
    pub frequency_hz: f32,
    pub duration_seconds: f32,
}

#[derive(Clone, Debug)]
pub struct SignaturePattern {
    pub name: String,
    pub frequency_hz: f32,
    pub occurrence_count: usize,
    pub confidence: f32,
    pub features: Vec<(String, f32)>, // (feature_name, importance_0_to_1)
}

#[derive(Clone, Debug)]
pub struct ClusterNode {
    pub name: String,
    pub size: usize,
    pub coherence: f32, // 0.0 = low, 1.0 = high
    pub x_position: f32,
    pub y_position: f32,
}

#[derive(Clone, Debug)]
pub struct CorrelationEdge {
    pub event_a_idx: usize,
    pub event_b_idx: usize,
    pub correlation_type: u32, // 0=temporal, 1=spectral, 2=spatial
    pub strength: f32,         // 0.0-1.0
}

/// Generate mock temporal events for scatter plot
pub fn generate_mock_events() -> Vec<AnalysisEvent> {
    vec![
        AnalysisEvent {
            timestamp_weeks: 1.0,
            intensity: 0.3,
            cluster_type: 0, // aggressive
            frequency_hz: 1500.0,
            duration_seconds: 30.0,
        },
        AnalysisEvent {
            timestamp_weeks: 3.5,
            intensity: 0.6,
            cluster_type: 0,
            frequency_hz: 2000.0,
            duration_seconds: 45.0,
        },
        AnalysisEvent {
            timestamp_weeks: 5.0,
            intensity: 0.8,
            cluster_type: 1, // subtle
            frequency_hz: 40000.0,
            duration_seconds: 20.0,
        },
        AnalysisEvent {
            timestamp_weeks: 8.0,
            intensity: 0.4,
            cluster_type: 2, // ongoing
            frequency_hz: 100.0,
            duration_seconds: 120.0,
        },
        AnalysisEvent {
            timestamp_weeks: 12.0,
            intensity: 0.7,
            cluster_type: 0,
            frequency_hz: 3000.0,
            duration_seconds: 60.0,
        },
        AnalysisEvent {
            timestamp_weeks: 15.5,
            intensity: 0.5,
            cluster_type: 1,
            frequency_hz: 50000.0,
            duration_seconds: 25.0,
        },
        AnalysisEvent {
            timestamp_weeks: 20.0,
            intensity: 0.9,
            cluster_type: 0,
            frequency_hz: 1800.0,
            duration_seconds: 90.0,
        },
        AnalysisEvent {
            timestamp_weeks: 24.0,
            intensity: 0.2,
            cluster_type: 3, // background
            frequency_hz: 500.0,
            duration_seconds: 180.0,
        },
    ]
}

/// Generate mock attack signatures for heatmap
pub fn generate_mock_signatures() -> Vec<SignaturePattern> {
    vec![
        SignaturePattern {
            name: "Ultrasonic Jammer (40 kHz)".to_string(),
            frequency_hz: 40000.0,
            occurrence_count: 15,
            confidence: 0.92,
            features: vec![
                ("Audio Coherence".to_string(), 0.95),
                ("RF Activity".to_string(), 0.15),
                ("Spatial Geometry".to_string(), 0.35),
                ("Polarization".to_string(), 0.20),
                ("Harmonic Series".to_string(), 0.85),
            ],
        },
        SignaturePattern {
            name: "Infrasound Pulse (10 Hz)".to_string(),
            frequency_hz: 10.0,
            occurrence_count: 12,
            confidence: 0.88,
            features: vec![
                ("Audio Coherence".to_string(), 0.75),
                ("RF Activity".to_string(), 0.10),
                ("Spatial Geometry".to_string(), 0.65),
                ("Polarization".to_string(), 0.30),
                ("Harmonic Series".to_string(), 0.45),
            ],
        },
        SignaturePattern {
            name: "Microwave Carrier (2.4 GHz)".to_string(),
            frequency_hz: 2_400_000_000.0,
            occurrence_count: 8,
            confidence: 0.75,
            features: vec![
                ("Audio Coherence".to_string(), 0.20),
                ("RF Activity".to_string(), 0.98),
                ("Spatial Geometry".to_string(), 0.70),
                ("Polarization".to_string(), 0.85),
                ("Harmonic Series".to_string(), 0.30),
            ],
        },
        SignaturePattern {
            name: "Audible Harassment (1.5 kHz)".to_string(),
            frequency_hz: 1500.0,
            occurrence_count: 20,
            confidence: 0.95,
            features: vec![
                ("Audio Coherence".to_string(), 0.99),
                ("RF Activity".to_string(), 0.05),
                ("Spatial Geometry".to_string(), 0.25),
                ("Polarization".to_string(), 0.10),
                ("Harmonic Series".to_string(), 0.90),
            ],
        },
        SignaturePattern {
            name: "Acoustic Echoing (500 Hz)".to_string(),
            frequency_hz: 500.0,
            occurrence_count: 10,
            confidence: 0.82,
            features: vec![
                ("Audio Coherence".to_string(), 0.88),
                ("RF Activity".to_string(), 0.08),
                ("Spatial Geometry".to_string(), 0.50),
                ("Polarization".to_string(), 0.15),
                ("Harmonic Series".to_string(), 0.75),
            ],
        },
    ]
}

/// Generate mock hierarchical clusters for dendrogram
pub fn generate_mock_clusters() -> Vec<ClusterNode> {
    vec![
        ClusterNode {
            name: "All Events (n=80)".to_string(),
            size: 80,
            coherence: 0.55,
            x_position: 0.50,
            y_position: 0.05,
        },
        ClusterNode {
            name: "Aggressive (n=35)".to_string(),
            size: 35,
            coherence: 0.82,
            x_position: 0.25,
            y_position: 0.35,
        },
        ClusterNode {
            name: "Subtle (n=25)".to_string(),
            size: 25,
            coherence: 0.68,
            x_position: 0.75,
            y_position: 0.35,
        },
        ClusterNode {
            name: "Ongoing (n=12)".to_string(),
            size: 12,
            coherence: 0.45,
            x_position: 0.50,
            y_position: 0.35,
        },
        ClusterNode {
            name: "Ultrasonic Sub-cluster (n=18)".to_string(),
            size: 18,
            coherence: 0.90,
            x_position: 0.15,
            y_position: 0.65,
        },
        ClusterNode {
            name: "Audible Sub-cluster (n=17)".to_string(),
            size: 17,
            coherence: 0.85,
            x_position: 0.35,
            y_position: 0.65,
        },
    ]
}

/// Generate mock correlation edges for network graph
pub fn generate_mock_correlations() -> Vec<CorrelationEdge> {
    vec![
        CorrelationEdge {
            event_a_idx: 0,
            event_b_idx: 1,
            correlation_type: 0, // temporal
            strength: 0.92,
        },
        CorrelationEdge {
            event_a_idx: 1,
            event_b_idx: 2,
            correlation_type: 1, // spectral
            strength: 0.78,
        },
        CorrelationEdge {
            event_a_idx: 2,
            event_b_idx: 3,
            correlation_type: 2, // spatial
            strength: 0.65,
        },
        CorrelationEdge {
            event_a_idx: 3,
            event_b_idx: 4,
            correlation_type: 0, // temporal
            strength: 0.55,
        },
        CorrelationEdge {
            event_a_idx: 4,
            event_b_idx: 5,
            correlation_type: 1, // spectral
            strength: 0.72,
        },
        CorrelationEdge {
            event_a_idx: 0,
            event_b_idx: 4,
            correlation_type: 2, // spatial
            strength: 0.68,
        },
        CorrelationEdge {
            event_a_idx: 1,
            event_b_idx: 5,
            correlation_type: 0, // temporal
            strength: 0.58,
        },
    ]
}

/// Utility: convert cluster type to color hex code
pub fn cluster_type_color(cluster_type: u32) -> &'static str {
    match cluster_type {
        0 => "#ff4040", // red (aggressive)
        1 => "#0099ff", // blue (subtle)
        2 => "#ffff00", // yellow (ongoing)
        3 => "#888888", // gray (background)
        _ => "#cccccc", // default
    }
}

/// Utility: convert cluster type to name
pub fn cluster_type_name(cluster_type: u32) -> &'static str {
    match cluster_type {
        0 => "Aggressive",
        1 => "Subtle",
        2 => "Ongoing",
        3 => "Background",
        _ => "Unknown",
    }
}

/// Utility: convert correlation type to color hex code
pub fn correlation_type_color(corr_type: u32) -> &'static str {
    match corr_type {
        0 => "#ff4040", // red (temporal)
        1 => "#0099ff", // blue (spectral)
        2 => "#00ff00", // green (spatial)
        _ => "#888888", // gray (unknown)
    }
}

/// Utility: convert correlation type to name
pub fn correlation_type_name(corr_type: u32) -> &'static str {
    match corr_type {
        0 => "Temporal",
        1 => "Spectral",
        2 => "Spatial",
        _ => "Unknown",
    }
}

/// Utility: heat map color from importance [0.0, 1.0]
/// Blue (0.0) → Red (0.5) → White (1.0)
pub fn heat_map_color(importance: f32) -> String {
    let clamped = importance.clamp(0.0, 1.0);
    if clamped < 0.5 {
        let r = clamped * 2.0;
        let b = 1.0 - clamped * 2.0;
        format!("#{:02x}{:02x}{:02x}", (r * 255.0) as u8, 0, (b * 255.0) as u8)
    } else {
        let r = 1.0;
        let gb = (clamped - 0.5) * 2.0;
        format!(
            "#{:02x}{:02x}{:02x}",
            255,
            (gb * 255.0) as u8,
            (gb * 255.0) as u8
        )
    }
}

/// Utility: coherence color
pub fn coherence_color(coherence: f32) -> &'static str {
    let clamped = coherence.clamp(0.0, 1.0);
    if clamped < 0.3 {
        "#ff6666" // weak red
    } else if clamped < 0.7 {
        "#ffff66" // yellow
    } else {
        "#66ff66" // strong green
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_data_generation() {
        let events = generate_mock_events();
        let sigs = generate_mock_signatures();
        let clusters = generate_mock_clusters();
        let corrs = generate_mock_correlations();

        assert!(!events.is_empty());
        assert!(!sigs.is_empty());
        assert!(!clusters.is_empty());
        assert!(!corrs.is_empty());

        // Verify event intensity is in [0, 1]
        for event in &events {
            assert!(event.intensity >= 0.0 && event.intensity <= 1.0);
        }

        // Verify signature features are in [0, 1]
        for sig in &sigs {
            for (_, importance) in &sig.features {
                assert!(importance >= &0.0 && importance <= &1.0);
            }
        }

        // Verify cluster coherence is in [0, 1]
        for cluster in &clusters {
            assert!(cluster.coherence >= 0.0 && cluster.coherence <= 1.0);
        }

        // Verify correlation strength is in [0, 1]
        for corr in &corrs {
            assert!(corr.strength >= 0.0 && corr.strength <= 1.0);
        }
    }

    #[test]
    fn test_color_utilities() {
        // Test cluster colors
        assert_eq!(cluster_type_color(0), "#ff4040");
        assert_eq!(cluster_type_color(1), "#0099ff");
        assert_eq!(cluster_type_color(2), "#ffff00");
        assert_eq!(cluster_type_color(3), "#888888");

        // Test correlation colors
        assert_eq!(correlation_type_color(0), "#ff4040");
        assert_eq!(correlation_type_color(1), "#0099ff");
        assert_eq!(correlation_type_color(2), "#00ff00");

        // Test heat map color generation
        let color_min = heat_map_color(0.0);
        let color_mid = heat_map_color(0.5);
        let color_max = heat_map_color(1.0);

        assert!(!color_min.is_empty());
        assert!(!color_mid.is_empty());
        assert!(!color_max.is_empty());
    }
}
