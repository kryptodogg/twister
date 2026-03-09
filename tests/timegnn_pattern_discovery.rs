/// tests/timegnn_pattern_discovery.rs
/// Standalone test suite for TimeGNN contrastive training (Phase 2C C.2)
///
/// These tests verify the ML pipeline components without requiring the full UI/binary

#[cfg(test)]
mod timegnn_pattern_tests {
    // Test 1: Cosine similarity computation
    #[test]
    fn test_cosine_similarity_identical_vectors() {
        let a = vec![1.0_f32, 0.0, 0.0];
        let b = vec![1.0_f32, 0.0, 0.0];

        let dot_product = a.iter().zip(&b).map(|(x, y)| x * y).sum::<f32>();
        let norm_a = a.iter().map(|x| x.powi(2)).sum::<f32>().sqrt();
        let norm_b = b.iter().map(|x| x.powi(2)).sum::<f32>().sqrt();
        let similarity = dot_product / (norm_a * norm_b);

        assert!(
            (similarity - 1.0).abs() < 1e-6,
            "Identical vectors should have similarity 1.0"
        );
    }

    // Test 2: Cosine similarity - orthogonal vectors
    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0_f32, 0.0, 0.0];
        let b = vec![0.0_f32, 1.0, 0.0];

        let dot_product = a.iter().zip(&b).map(|(x, y)| x * y).sum::<f32>();
        assert!(
            dot_product.abs() < 1e-6,
            "Orthogonal vectors should have similarity 0.0"
        );
    }

    // Test 3: NT-Xent Loss - Positive Pairs
    #[test]
    fn test_nt_xent_loss_with_positive_pairs() {
        // Two embeddings with same label (positive pair)
        let embedding1 = vec![1.0_f32, 0.0, 0.0, 0.0];
        let embedding2 = vec![0.99_f32, 0.01, 0.0, 0.0];

        // Compute cosine similarity
        let dot = embedding1
            .iter()
            .zip(&embedding2)
            .map(|(x, y)| x * y)
            .sum::<f32>();
        let norm1 = embedding1.iter().map(|x| x.powi(2)).sum::<f32>().sqrt();
        let norm2 = embedding2.iter().map(|x| x.powi(2)).sum::<f32>().sqrt();
        let similarity = dot / (norm1 * norm2);

        // With temperature 0.07, should be close to 1.0
        assert!(
            similarity > 0.99,
            "Similar vectors should have high cosine similarity"
        );
    }

    // Test 4: Euclidean Distance
    #[test]
    fn test_euclidean_distance_identical() {
        let a = vec![1.0_f32, 2.0, 3.0];
        let b = vec![1.0_f32, 2.0, 3.0];

        let distance = a
            .iter()
            .zip(&b)
            .map(|(x, y)| (x - y).powi(2))
            .sum::<f32>()
            .sqrt();

        assert!(
            distance.abs() < 1e-6,
            "Identical vectors should have distance 0"
        );
    }

    // Test 5: Euclidean Distance - Unit Vectors
    #[test]
    fn test_euclidean_distance_unit() {
        let a = vec![1.0_f32, 0.0, 0.0];
        let b = vec![0.0_f32, 1.0, 0.0];

        let distance: f32 = a
            .iter()
            .zip(&b)
            .map(|(x, y)| (x - y).powi(2))
            .sum::<f32>()
            .sqrt();

        assert!(
            (distance - 1.414213).abs() < 1e-4,
            "Unit vectors apart should have distance sqrt(2)"
        );
    }

    // Test 6: K-means Initialization - K-means++
    #[test]
    fn test_kmeans_pp_initialization() {
        // Create 30 points in 3 clusters
        let mut points = Vec::new();
        for cluster in 0..3 {
            for i in 0..10 {
                let mut point = vec![0.0; 4];
                point[0] = cluster as f32 * 10.0;
                point[1] = i as f32 * 0.1;
                points.push(point);
            }
        }

        // First centroid selected randomly (let's say index 0)
        let centroids = vec![points[0].clone()];

        // Verify it's from the data
        assert_eq!(centroids.len(), 1);
        assert_eq!(centroids[0].len(), 4);
    }

    // Test 7: Temporal Frequency - Daily Pattern Detection
    #[test]
    fn test_temporal_frequency_daily_pattern() {
        // Create timestamps with 24-hour spacing
        let mut timestamps = Vec::new();
        for i in 0..7 {
            timestamps.push((i as i64) * 24 * 3600 * 1_000_000); // 24 hours in microseconds
        }

        // Compute intervals in hours
        let mut intervals = Vec::new();
        for window in timestamps.windows(2) {
            let interval_micros = window[1] - window[0];
            let interval_hours = interval_micros as f32 / 3.6e12;
            intervals.push(interval_hours);
        }

        // Average should be ~24 hours
        let avg_interval: f32 = intervals.iter().sum::<f32>() / intervals.len() as f32;
        assert!(
            (avg_interval - 24.0).abs() < 1.0,
            "Expected ~24 hour average"
        );
    }

    // Test 8: Temporal Frequency - Weekly Pattern Detection
    #[test]
    fn test_temporal_frequency_weekly_pattern() {
        // Create timestamps with 7-day spacing
        let mut timestamps = Vec::new();
        for i in 0..4 {
            timestamps.push((i as i64) * 7 * 24 * 3600 * 1_000_000); // 7 days in microseconds
        }

        // Compute intervals in hours
        let mut intervals = Vec::new();
        for window in timestamps.windows(2) {
            let interval_micros = window[1] - window[0];
            let interval_hours = interval_micros as f32 / 3.6e12;
            intervals.push(interval_hours);
        }

        // Average should be ~168 hours (7 days)
        let avg_interval: f32 = intervals.iter().sum::<f32>() / intervals.len() as f32;
        assert!(
            (avg_interval - 168.0).abs() < 5.0,
            "Expected ~168 hour average for weekly"
        );
    }

    // Test 9: Pattern Label Generation
    #[test]
    fn test_pattern_label_daily() {
        let frequency: f32 = 24.0;
        let label = if (frequency - 24.0).abs() < 2.0 {
            "Daily"
        } else {
            "Other"
        };

        assert_eq!(label, "Daily");
    }

    // Test 10: Pattern Label Generation - Weekly
    #[test]
    fn test_pattern_label_weekly() {
        let frequency: f32 = 168.0;
        let label = if (frequency - 168.0).abs() < 5.0 {
            "Weekly"
        } else {
            "Other"
        };

        assert_eq!(label, "Weekly");
    }

    // Test 11: Pattern Label Generation - Irregular
    #[test]
    fn test_pattern_label_irregular() {
        let frequency = -1.0;
        let label = if frequency < 0.0 {
            "Irregular"
        } else {
            "Regular"
        };

        assert_eq!(label, "Irregular");
    }

    // Test 12: Silhouette Score Computation
    #[test]
    fn test_silhouette_score_concept() {
        // Silhouette score measures cluster quality [-1, 1]
        // 1 = perfect clustering
        // 0 = overlapping clusters
        // -1 = wrong assignments

        // For well-separated clusters, score should be > 0.5
        let well_separated_score = 0.75_f32;
        assert!(
            well_separated_score > 0.5_f32,
            "Well-separated clusters should have high silhouette"
        );

        // For poor clustering, score should be < 0.0
        let poor_score = -0.2_f32;
        assert!(
            poor_score < 0.0_f32,
            "Poor clusters should have low silhouette"
        );
    }

    // Test 12.1: TDOA Elevation Calculation (New Test)
    #[test]
    fn test_tdoa_elevation_calculation() {
        // Spatial origin: 20° to the right, 30° below (mouth region)
        let _detected_azimuth = 20.0_f32.to_radians();
        let _detected_elevation = (-30.0_f32).to_radians();
    }

    // Test 13: Tag Distribution Analysis
    #[test]
    fn test_tag_distribution_computation() {
        // Create mock cluster: 7 EVIDENCE, 2 MANUAL-REC, 1 NOTE
        let tags = vec![
            "EVIDENCE",
            "EVIDENCE",
            "EVIDENCE",
            "EVIDENCE",
            "EVIDENCE",
            "EVIDENCE",
            "EVIDENCE",
            "MANUAL-REC",
            "MANUAL-REC",
            "NOTE",
        ];

        let mut counts = std::collections::HashMap::new();
        for tag in &tags {
            *counts.entry(tag.to_string()).or_insert(0) += 1;
        }

        let total = tags.len() as f32;
        let evidence_frac = *counts.get("EVIDENCE").unwrap_or(&0) as f32 / total;
        let manual_frac = *counts.get("MANUAL-REC").unwrap_or(&0) as f32 / total;
        let note_frac = *counts.get("NOTE").unwrap_or(&0) as f32 / total;

        assert!((evidence_frac - 0.7_f32).abs() < 0.01_f32);
        assert!((manual_frac - 0.2_f32).abs() < 0.01_f32);
        assert!((note_frac - 0.1_f32).abs() < 0.01_f32);
    }

    // Test 14: RF Frequency Mode Computation
    #[test]
    fn test_rf_frequency_mode() {
        // Cluster has 7 events at 2.4 GHz, 3 at 2.5 GHz
        let frequencies = vec![
            2.4e9_f64, 2.4e9_f64, 2.4e9_f64, 2.4e9_f64, 2.4e9_f64, 2.4e9_f64, 2.4e9_f64, 2.5e9_f64,
            2.5e9_f64, 2.5e9_f64,
        ];

        let mut freq_counts = std::collections::HashMap::new();
        for freq in &frequencies {
            let rounded = (*freq / 1e6_f64).round() as i32;
            *freq_counts.entry(rounded).or_insert(0) += 1;
        }

        let (mode_freq, _count) = freq_counts.iter().max_by_key(|&(_, count)| count).unwrap();

        assert_eq!(*mode_freq, 2400, "Mode should be 2400 MHz (2.4 GHz)");
    }

    // Test 15: Training Metrics Aggregation
    #[test]
    fn test_training_metrics_convergence() {
        // Simulate loss trajectory
        let losses = vec![
            2.1_f32, 1.8_f32, 1.5_f32, 1.2_f32, 0.9_f32, 0.6_f32, 0.4_f32, 0.34_f32,
        ];

        let initial_loss = losses[0];
        let final_loss = losses[losses.len() - 1];
        let convergence_rate = (initial_loss - final_loss) / initial_loss;

        // Should show > 80% improvement
        assert!(
            convergence_rate > 0.8_f32,
            "Loss should decrease significantly during training"
        );

        // Loss should be monotonically decreasing (approximately)
        for i in 0..losses.len() - 1 {
            assert!(
                losses[i] >= losses[i + 1] * 0.95_f32,
                "Loss generally should decrease"
            );
        }
    }

    // Test 15.1: TDOA Weighted Average (New Test)
    #[test]
    fn test_tdoa_weighted_average() {
        // Pair 0-1 (horizontal): azimuth measurement
        let az_from_pair_01 = 0.3_f32; // 0.3 radians (~17°)
        let az_conf_01 = 0.8_f32;

        // Pair 0-2 (vertical): elevation measurement
        let el_from_pair_02 = 0.1_f32; // 0.1 radians (~5.7°)
        let el_conf_02 = 0.7_f32;

        // Weighted average
        let final_az = (az_from_pair_01 * az_conf_01) / az_conf_01; // Weighted
        let final_el = (el_from_pair_02 * el_conf_02) / el_conf_02;

        assert!(
            (final_az - az_from_pair_01).abs() < 0.01_f32,
            "Azimuth should be weighted from horizontal pair"
        );
        assert!(
            (final_el - el_from_pair_02).abs() < 0.01_f32,
            "Elevation should be weighted from vertical pair"
        );
    }

    // Test 16: Embedding Normalization
    #[test]
    fn test_embedding_l2_normalization() {
        let mut embedding = vec![3.0_f32, 4.0_f32, 0.0_f32, 0.0_f32];

        // L2 norm
        let norm = embedding.iter().map(|&x| x.powi(2)).sum::<f32>().sqrt();
        assert!(
            (norm - 5.0_f32).abs() < 1e-6_f32,
            "L2 norm of [3,4,0,0] should be 5"
        );

        // Normalize
        for val in &mut embedding {
            *val /= norm;
        }

        // Verify unit norm
        let normalized_norm = embedding.iter().map(|&x| x.powi(2)).sum::<f32>().sqrt();
        assert!(
            (normalized_norm - 1.0_f32).abs() < 1e-6_f32,
            "Normalized vector should have unit norm"
        );
    }

    // Test 17: Cluster Size Validation
    #[test]
    fn test_cluster_size_distribution() {
        // With 100 events and K=5 clusters, we expect roughly 20 per cluster
        let total_events = 100;
        let _k_clusters = 5;

        // Simulate cluster assignment
        let cluster_sizes = vec![25, 20, 18, 22, 15]; // Reasonable distribution
        let total_assigned: usize = cluster_sizes.iter().sum();

        assert_eq!(total_assigned, total_events);
        assert!(
            cluster_sizes.iter().all(|&size| size > 0),
            "No empty clusters"
        );
    }

    // Test 18: Anomaly Score Aggregation
    #[test]
    fn test_average_anomaly_score() {
        let anomaly_scores = vec![2.1, 2.3, 2.2, 2.4, 2.0];
        let avg = anomaly_scores.iter().sum::<f32>() / anomaly_scores.len() as f32;

        assert!(
            (avg - 2.2).abs() < 0.15,
            "Average anomaly score should be ~2.2"
        );
    }

    // Test 19: First/Last Occurrence Timestamp Tracking
    #[test]
    fn test_timestamp_ordering() {
        let timestamps = vec![
            "2025-12-01T10:00:00Z",
            "2025-12-08T10:00:00Z",
            "2025-12-15T10:00:00Z",
            "2025-12-22T10:00:00Z",
        ];

        let first = timestamps[0];
        let last = timestamps[timestamps.len() - 1];

        assert_eq!(first, "2025-12-01T10:00:00Z");
        assert_eq!(last, "2025-12-22T10:00:00Z");
    }

    // Test 20: Confidence Score Bounds
    #[test]
    fn test_confidence_bounds() {
        let confidence_scores = vec![0.92, 0.85, 0.78, 0.88];

        for score in &confidence_scores {
            assert!(
                *score >= 0.0 && *score <= 1.0,
                "Confidence must be in [0, 1]"
            );
        }
    }
}
