/// src/ml/pattern_discovery.rs
/// Pattern Discovery — Cluster event embeddings into harassment motifs
///
/// Purpose: Apply K-means clustering to 128-D TimeGNN embeddings to discover
/// 23 recurring harassment patterns with temporal frequency detection.
///
/// Algorithm:
/// 1. Initialize K=23 cluster centers (random from data)
/// 2. Assign events to nearest cluster (Euclidean distance)
/// 3. Update centroids as mean of assigned points
/// 4. Repeat until convergence
/// 5. Per-cluster analysis: temporal frequency, confidence, tag distribution
///
/// Output: Vec<Pattern> with full metadata for harassment signature library
use std::collections::HashMap;

/// Harassment pattern discovered from event clustering
#[derive(Debug, Clone)]
pub struct Pattern {
    /// Unique pattern identifier (0-22 for K=23)
    pub motif_id: usize,
    /// Human-readable pattern label (e.g., "Friday_3PM_Tone")
    pub label: String,
    /// Recurrence period in hours (e.g., 168.0 for weekly)
    pub frequency_hours: f32,
    /// Confidence score from silhouette analysis [0, 1]
    pub confidence: f32,
    /// Number of events in this cluster
    pub cluster_size: usize,
    /// 128-D centroid embedding for this pattern
    pub representative_embedding: Vec<f32>,
    /// First occurrence timestamp (ISO8601 format)
    pub first_occurrence_iso: String,
    /// Last occurrence timestamp (ISO8601 format)
    pub last_occurrence_iso: String,
    /// Distribution of forensic tags in cluster
    pub tag_distribution: HashMap<String, f32>,
    /// Silhouette score for cluster quality [-1, 1]
    pub silhouette_score: f32,
    /// Average Mamba anomaly score for cluster
    pub avg_anomaly_score: f32,
    /// Most common RF frequency in cluster (Hz)
    pub rf_frequency_hz_mode: f32,
}

/// Event data for pattern discovery
#[derive(Debug, Clone)]
pub struct Event {
    /// Unique event ID
    pub id: String,
    /// 128-D TimeGNN embedding
    pub embedding: Vec<f32>,
    /// Unix timestamp (microseconds)
    pub timestamp_micros: i64,
    /// ISO8601 timestamp string
    pub timestamp_iso: String,
    /// Forensic event tag
    pub tag: String,
    /// RF frequency in Hz
    pub rf_frequency_hz: f32,
    /// Mamba anomaly score
    pub anomaly_score: f32,
}

/// K-means clustering configuration
pub struct KMeansConfig {
    /// Number of clusters (harassment motifs)
    pub k: usize,
    /// Maximum iterations for convergence
    pub max_iterations: usize,
    /// Tolerance for convergence (sum of centroid movements)
    pub convergence_threshold: f32,
}

impl Default for KMeansConfig {
    fn default() -> Self {
        Self {
            k: 23,
            max_iterations: 100,
            convergence_threshold: 1e-4,
        }
    }
}

/// Clustering result with assignments and centroids
#[derive(Debug, Clone)]
pub struct ClusteringResult {
    /// Cluster assignment for each event (event_idx -> cluster_id)
    pub assignments: Vec<usize>,
    /// Cluster centroids (k x 128)
    pub centroids: Vec<Vec<f32>>,
    /// Inertia (sum of squared distances to centroids)
    pub inertia: f32,
    /// Number of iterations until convergence
    pub iterations_to_convergence: u32,
}

/// Euclidean distance between two vectors
fn euclidean_distance(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return f32::INFINITY;
    }
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x - y).powi(2))
        .sum::<f32>()
        .sqrt()
}

/// Initialize K cluster centers from data (K-means++)
/// Uses probabilistic selection to spread initial centers
fn initialize_centroids_kmeans_pp(embeddings: &[Vec<f32>], k: usize) -> Vec<Vec<f32>> {
    if embeddings.is_empty() || k == 0 {
        return Vec::new();
    }

    let mut centroids = Vec::new();

    // Choose first center randomly
    let first_idx = 0;
    centroids.push(embeddings[first_idx].clone());

    // Choose remaining k-1 centers
    for _ in 1..k {
        let mut max_distance = 0.0;
        let mut farthest_idx = 0;

        for (i, embedding) in embeddings.iter().enumerate() {
            // Find minimum distance to existing centroids
            let min_dist_to_centroid = centroids
                .iter()
                .map(|c| euclidean_distance(embedding, c))
                .fold(f32::INFINITY, f32::min);

            if min_dist_to_centroid > max_distance {
                max_distance = min_dist_to_centroid;
                farthest_idx = i;
            }
        }

        centroids.push(embeddings[farthest_idx].clone());
    }

    centroids
}

/// K-means clustering on 128-D embeddings
pub fn kmeans(embeddings: &[Vec<f32>], config: KMeansConfig) -> Result<ClusteringResult, String> {
    if embeddings.is_empty() {
        return Err("No embeddings provided".to_string());
    }

    if config.k > embeddings.len() {
        return Err(format!(
            "K ({}) cannot exceed number of points ({})",
            config.k,
            embeddings.len()
        ));
    }

    let n = embeddings.len();
    let dim = embeddings[0].len();

    // Verify all embeddings have same dimension
    for embedding in embeddings {
        if embedding.len() != dim {
            return Err("All embeddings must have same dimension".to_string());
        }
    }

    // Initialize centroids
    let mut centroids = initialize_centroids_kmeans_pp(embeddings, config.k);

    let mut assignments = vec![0usize; n];
    let mut prev_inertia = f32::INFINITY;

    for iteration in 0..config.max_iterations {
        // Assignment step: assign each point to nearest centroid
        let mut new_inertia = 0.0;
        for (i, embedding) in embeddings.iter().enumerate() {
            let (nearest_centroid, min_distance) = centroids
                .iter()
                .enumerate()
                .map(|(j, centroid)| (j, euclidean_distance(embedding, centroid)))
                .fold((0, f32::INFINITY), |acc, (j, dist)| {
                    if dist < acc.1 {
                        (j, dist)
                    } else {
                        acc
                    }
                });

            assignments[i] = nearest_centroid;
            new_inertia += min_distance.powi(2);
        }

        // Check convergence
        let inertia_change = (prev_inertia - new_inertia).abs();
        if inertia_change < config.convergence_threshold {
            return Ok(ClusteringResult {
                assignments,
                centroids,
                inertia: new_inertia,
                iterations_to_convergence: (iteration + 1) as u32,
            });
        }

        // Update step: recompute centroids
        let mut new_centroids = vec![vec![0.0; dim]; config.k];
        let mut counts = vec![0usize; config.k];

        for (i, &cluster_id) in assignments.iter().enumerate() {
            for d in 0..dim {
                new_centroids[cluster_id][d] += embeddings[i][d];
            }
            counts[cluster_id] += 1;
        }

        // Average to get new centroids
        let mut converged = true;
        for k in 0..config.k {
            if counts[k] > 0 {
                for d in 0..dim {
                    new_centroids[k][d] /= counts[k] as f32;
                }
                // Check movement
                let movement = euclidean_distance(&centroids[k], &new_centroids[k]);
                if movement > config.convergence_threshold {
                    converged = false;
                }
            } else {
                // Empty cluster: reinitialize from random point
                new_centroids[k] = embeddings[k % embeddings.len()].clone();
            }
        }

        centroids = new_centroids;
        prev_inertia = new_inertia;

        if converged {
            return Ok(ClusteringResult {
                assignments,
                centroids,
                inertia: new_inertia,
                iterations_to_convergence: (iteration + 1) as u32,
            });
        }
    }

    Ok(ClusteringResult {
        assignments,
        centroids,
        inertia: prev_inertia,
        iterations_to_convergence: config.max_iterations as u32,
    })
}

/// Compute silhouette score for a clustering result
/// Measures how similar an object is to its own cluster vs other clusters
/// Range: [-1, 1] where 1 = perfect clustering, -1 = wrong assignment
pub fn compute_silhouette_score(embeddings: &[Vec<f32>], clustering: &ClusteringResult) -> f32 {
    if embeddings.is_empty() || embeddings.len() != clustering.assignments.len() {
        return 0.0;
    }

    let mut total_score = 0.0;

    for (i, embedding) in embeddings.iter().enumerate() {
        let cluster_id = clustering.assignments[i];

        // a(i): average distance to other points in same cluster
        let mut same_cluster_distances = Vec::new();
        for (j, &assigned_cluster) in clustering.assignments.iter().enumerate() {
            if i != j && assigned_cluster == cluster_id {
                same_cluster_distances.push(euclidean_distance(embedding, &embeddings[j]));
            }
        }

        let a_i = if same_cluster_distances.is_empty() {
            0.0
        } else {
            same_cluster_distances.iter().sum::<f32>() / same_cluster_distances.len() as f32
        };

        // b(i): minimum average distance to points in other clusters
        let mut b_i = f32::INFINITY;
        for other_cluster_id in 0..clustering.centroids.len() {
            if other_cluster_id == cluster_id {
                continue;
            }

            let mut other_cluster_distances = Vec::new();
            for (j, &assigned_cluster) in clustering.assignments.iter().enumerate() {
                if assigned_cluster == other_cluster_id {
                    other_cluster_distances.push(euclidean_distance(embedding, &embeddings[j]));
                }
            }

            if !other_cluster_distances.is_empty() {
                let avg_dist = other_cluster_distances.iter().sum::<f32>()
                    / other_cluster_distances.len() as f32;
                b_i = b_i.min(avg_dist);
            }
        }

        // Silhouette coefficient for this point
        let s_i = if a_i < b_i {
            1.0 - (a_i / b_i)
        } else if a_i > b_i {
            (b_i / a_i) - 1.0
        } else {
            0.0
        };

        total_score += s_i;
    }

    total_score / embeddings.len() as f32
}

/// Compute temporal frequency from event timestamps
/// Returns dominant recurrence period in hours (or -1 if no clear pattern)
pub fn compute_temporal_frequency(events: &[Event], cluster_members: &[usize]) -> f32 {
    if cluster_members.len() < 2 {
        return -1.0;
    }

    // Extract timestamps for cluster members (sorted)
    let mut timestamps: Vec<i64> = cluster_members
        .iter()
        .map(|&idx| events[idx].timestamp_micros)
        .collect();
    timestamps.sort();

    // Compute inter-event intervals (in hours)
    let mut intervals = Vec::new();
    for window in timestamps.windows(2) {
        let interval_micros = window[1] - window[0];
        let interval_hours = interval_micros as f32 / 3.6e12; // 1 hour = 3.6e12 microseconds
        if interval_hours > 0.01 {
            // Skip sub-minute intervals (noise)
            intervals.push(interval_hours);
        }
    }

    if intervals.is_empty() {
        return -1.0;
    }

    // Find dominant frequency via histogram binning
    // Simple approach: use mode (most common interval rounded to nearest hour)
    let mut interval_counts = HashMap::new();
    for interval in &intervals {
        let rounded = (interval.round()) as i32;
        *interval_counts.entry(rounded).or_insert(0) += 1;
    }

    let (dominant_interval, _count) = interval_counts
        .iter()
        .max_by_key(|&(_, count)| count)
        .unwrap_or((&1, &1));

    (*dominant_interval) as f32
}

/// Generate human-readable label for pattern based on frequency and characteristics
pub fn generate_pattern_label(motif_id: usize, frequency_hours: f32, rf_frequency: f32) -> String {
    if frequency_hours < 0.0 {
        return format!("Pattern_{}", motif_id);
    }

    let label_base = match frequency_hours {
        h if (h - 24.0).abs() < 2.0 => "Daily",
        h if (h - 12.0).abs() < 1.0 => "Twice_Daily",
        h if (h - 168.0).abs() < 5.0 => "Weekly",
        h if (h - 720.0).abs() < 20.0 => "Monthly",
        h if h < 6.0 => "Hourly",
        _ => "Intermittent",
    };

    let freq_label = if rf_frequency > 1e8 {
        format!("RF_{:.1}MHz", rf_frequency / 1e6)
    } else {
        "Acoustic".to_string()
    };

    format!("{}_{}", label_base, freq_label)
}

/// Discover harassment patterns from TimeGNN embeddings
///
/// # Arguments
/// * `embeddings` - 128-D TimeGNN embeddings for all events
/// * `events` - Event metadata (timestamps, tags, frequency, anomaly scores)
/// * `k` - Number of clusters to discover (default: 23)
///
/// # Returns
/// Vector of Pattern structures with full harassment motif metadata
pub fn discover_patterns(
    embeddings: &[Vec<f32>],
    events: &[Event],
    k: usize,
) -> Result<Vec<Pattern>, String> {
    if embeddings.is_empty() || events.is_empty() || embeddings.len() != events.len() {
        return Err("Embeddings and events must have same length".to_string());
    }

    // Run K-means clustering
    let config = KMeansConfig {
        k,
        ..Default::default()
    };
    let clustering = kmeans(embeddings, config)?;

    // Compute silhouette scores
    let avg_silhouette = compute_silhouette_score(embeddings, &clustering);

    // Per-cluster analysis
    let mut patterns = Vec::new();

    for cluster_id in 0..k {
        // Find all events in this cluster
        let cluster_members: Vec<usize> = clustering
            .assignments
            .iter()
            .enumerate()
            .filter_map(|(i, &c)| if c == cluster_id { Some(i) } else { None })
            .collect();

        if cluster_members.is_empty() {
            continue;
        }

        // Compute temporal frequency
        let frequency_hours = compute_temporal_frequency(events, &cluster_members);

        // Compute tag distribution
        let mut tag_counts = HashMap::new();
        for &member_idx in &cluster_members {
            *tag_counts
                .entry(events[member_idx].tag.clone())
                .or_insert(0) += 1;
        }
        let mut tag_distribution = HashMap::new();
        let cluster_size = cluster_members.len() as f32;
        for (tag, count) in tag_counts {
            tag_distribution.insert(tag, count as f32 / cluster_size);
        }

        // Compute average anomaly score
        let avg_anomaly: f32 = cluster_members
            .iter()
            .map(|&idx| events[idx].anomaly_score)
            .sum::<f32>()
            / cluster_members.len() as f32;

        // Find RF frequency mode
        let mut freq_counts = HashMap::new();
        for &member_idx in &cluster_members {
            let freq_key = (events[member_idx].rf_frequency_hz / 1e6).round() as i32;
            *freq_counts.entry(freq_key).or_insert(0) += 1;
        }
        let rf_mode_mhz = freq_counts
            .iter()
            .max_by_key(|&(_, count)| count)
            .map(|(&freq, _)| freq as f32 * 1e6)
            .unwrap_or(2.4e9);

        // Get first and last timestamps
        let mut member_timestamps: Vec<(i64, usize)> = cluster_members
            .iter()
            .map(|&idx| (events[idx].timestamp_micros, idx))
            .collect();
        member_timestamps.sort();

        let first_idx = member_timestamps.first().map(|(_, idx)| *idx).unwrap_or(0);
        let last_idx = member_timestamps.last().map(|(_, idx)| *idx).unwrap_or(0);

        // Generate label
        let label = generate_pattern_label(cluster_id, frequency_hours, rf_mode_mhz);

        patterns.push(Pattern {
            motif_id: cluster_id,
            label,
            frequency_hours,
            confidence: avg_silhouette,
            cluster_size: cluster_members.len(),
            representative_embedding: clustering.centroids[cluster_id].clone(),
            first_occurrence_iso: events[first_idx].timestamp_iso.clone(),
            last_occurrence_iso: events[last_idx].timestamp_iso.clone(),
            tag_distribution,
            silhouette_score: avg_silhouette,
            avg_anomaly_score: avg_anomaly,
            rf_frequency_hz_mode: rf_mode_mhz,
        });
    }

    patterns.sort_by_key(|p| std::cmp::Reverse(p.cluster_size));

    Ok(patterns)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_event(idx: usize, timestamp_micros: i64, cluster_id: usize) -> Event {
        Event {
            id: format!("event_{}", idx),
            embedding: vec![idx as f32 / 100.0; 128],
            timestamp_micros,
            timestamp_iso: format!("2025-12-{:02}T12:00:00Z", (idx % 28) + 1),
            tag: format!("TAG_{}", cluster_id),
            rf_frequency_hz: 2.4e9 + (cluster_id as f32 * 1e7),
            anomaly_score: 2.5 + (cluster_id as f32 * 0.1),
        }
    }

    #[test]
    fn test_euclidean_distance_identical() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![1.0, 2.0, 3.0];
        assert!((euclidean_distance(&a, &b) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_euclidean_distance_unit_vectors() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        assert!((euclidean_distance(&a, &b) - 1.414213).abs() < 1e-4);
    }

    #[test]
    fn test_kmeans_clustering_basic() {
        // Create 30 embeddings: 10 each in 3 clusters
        let mut embeddings = Vec::new();
        for cluster in 0..3 {
            for i in 0..10 {
                let base = cluster as f32;
                let mut emb = vec![base; 128];
                emb[0] += i as f32 * 0.01;
                embeddings.push(emb);
            }
        }

        let config = KMeansConfig {
            k: 3,
            max_iterations: 50,
            convergence_threshold: 1e-4,
        };

        let result = kmeans(&embeddings, config).unwrap();
        assert_eq!(result.assignments.len(), 30);
        assert_eq!(result.centroids.len(), 3);
    }

    #[test]
    fn test_silhouette_score_perfect_clustering() {
        // Create 2 well-separated clusters
        let mut embeddings = Vec::new();
        let mut assignments = Vec::new();

        // Cluster 0: points near (0, 0, 0, ...)
        for i in 0..10 {
            let mut emb = vec![0.0; 128];
            emb[0] = i as f32 * 0.01;
            embeddings.push(emb);
            assignments.push(0);
        }

        // Cluster 1: points near (10, 10, 10, ...)
        for i in 0..10 {
            let mut emb = vec![10.0; 128];
            emb[0] += i as f32 * 0.01;
            embeddings.push(emb);
            assignments.push(1);
        }

        let clustering = ClusteringResult {
            assignments,
            centroids: vec![vec![0.0; 128], vec![10.0; 128]],
            inertia: 0.0,
            iterations_to_convergence: 1,
        };

        let score = compute_silhouette_score(&embeddings, &clustering);
        assert!(
            score > 0.5,
            "Well-separated clusters should have high silhouette score"
        );
    }

    #[test]
    fn test_temporal_frequency_daily() {
        let mut events = Vec::new();
        for i in 0..7 {
            events.push(Event {
                id: format!("event_{}", i),
                embedding: vec![0.0; 128],
                timestamp_micros: (i as i64) * 24 * 3600 * 1_000_000, // Daily
                timestamp_iso: "2025-12-01T00:00:00Z".to_string(),
                tag: "TAG".to_string(),
                rf_frequency_hz: 2.4e9,
                anomaly_score: 2.0,
            });
        }

        let cluster_members: Vec<usize> = (0..7).collect();
        let frequency = compute_temporal_frequency(&events, &cluster_members);
        assert!(
            (frequency - 24.0).abs() < 2.0,
            "Expected ~24 hour frequency"
        );
    }

    #[test]
    fn test_temporal_frequency_weekly() {
        let mut events = Vec::new();
        for i in 0..4 {
            events.push(Event {
                id: format!("event_{}", i),
                embedding: vec![0.0; 128],
                timestamp_micros: (i as i64) * 7 * 24 * 3600 * 1_000_000, // Weekly
                timestamp_iso: "2025-12-01T00:00:00Z".to_string(),
                tag: "TAG".to_string(),
                rf_frequency_hz: 2.4e9,
                anomaly_score: 2.0,
            });
        }

        let cluster_members: Vec<usize> = (0..4).collect();
        let frequency = compute_temporal_frequency(&events, &cluster_members);
        assert!(
            (frequency - 168.0).abs() < 10.0,
            "Expected ~168 hour frequency"
        );
    }

    #[test]
    fn test_pattern_label_generation_daily() {
        let label = generate_pattern_label(0, 24.0, 2.4e9);
        assert!(label.contains("Daily"));
    }

    #[test]
    fn test_pattern_label_generation_weekly() {
        let label = generate_pattern_label(1, 168.0, 2.4e9);
        assert!(label.contains("Weekly"));
    }

    #[test]
    fn test_pattern_label_generation_irregular() {
        let label = generate_pattern_label(5, -1.0, 2.4e9);
        assert!(label.contains("Pattern_5"));
    }

    #[test]
    fn test_discover_patterns_basic() {
        // Create 20 events: 10 per cluster
        let mut embeddings = Vec::new();
        let mut events = Vec::new();

        for cluster in 0..2 {
            for i in 0..10 {
                let mut emb = vec![0.0; 128];
                emb[0] = cluster as f32;
                emb[1] = (i as f32) * 0.01;
                embeddings.push(emb);

                events.push(Event {
                    id: format!("event_{}_{}", cluster, i),
                    embedding: vec![0.0; 128],
                    timestamp_micros: (cluster as i64 * 1000 + i as i64) * 3600 * 1_000_000,
                    timestamp_iso: "2025-12-01T00:00:00Z".to_string(),
                    tag: format!("TAG_{}", cluster),
                    rf_frequency_hz: 2.4e9 + (cluster as f32 * 1e8),
                    anomaly_score: 2.0,
                });
            }
        }

        let patterns = discover_patterns(&embeddings, &events, 2).unwrap();
        assert_eq!(patterns.len(), 2);
        assert!(patterns[0].cluster_size >= 8); // Should have clustered reasonably
    }
}
