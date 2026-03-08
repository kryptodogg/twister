//! Chamfer Distance Loss for Point Cloud Geometry
//!
//! **Formula**:
//! ```
//! L_CD = (1/N) * Σ min_j ||p_i - q_j||² + (1/M) * Σ min_i ||q_j - p_i||²
//! ```
//! **Why important**: Handles unordered point clouds (permutation-invariant)

use std::error::Error;

/// Chamfer Distance reference specification
/// In production, this would be implemented as a GPU kernel for (N, M) point pairs
pub struct ChamferDistance;

impl ChamferDistance {
    /// Compute bidirectional nearest-neighbor distance
    /// Args: predicted (N, 3), ground_truth (M, 3) point clouds
    /// Returns: scalar loss value
    pub fn compute_loss(
        pred_points: &[(f32, f32, f32)],
        truth_points: &[(f32, f32, f32)],
    ) -> Result<f32, Box<dyn Error>> {
        if pred_points.is_empty() || truth_points.is_empty() {
            return Err("Empty point cloud".into());
        }

        // Forward direction: for each pred, find nearest truth
        let mut forward_loss = 0.0f32;
        for &(px, py, pz) in pred_points {
            let mut min_dist_sq = f32::INFINITY;
            for &(tx, ty, tz) in truth_points {
                let dx = px - tx;
                let dy = py - ty;
                let dz = pz - tz;
                let dist_sq = dx*dx + dy*dy + dz*dz;
                min_dist_sq = min_dist_sq.min(dist_sq);
            }
            forward_loss += min_dist_sq;
        }
        forward_loss /= pred_points.len() as f32;

        // Backward direction: for each truth, find nearest pred
        let mut backward_loss = 0.0f32;
        for &(tx, ty, tz) in truth_points {
            let mut min_dist_sq = f32::INFINITY;
            for &(px, py, pz) in pred_points {
                let dx = tx - px;
                let dy = ty - py;
                let dz = tz - pz;
                let dist_sq = dx*dx + dy*dy + dz*dz;
                min_dist_sq = min_dist_sq.min(dist_sq);
            }
            backward_loss += min_dist_sq;
        }
        backward_loss /= truth_points.len() as f32;

        // Combined: symmetric bidirectional loss
        Ok((forward_loss + backward_loss) / 2.0)
    }
}

/// Huber Loss for outlier robustness
/// Formula: 0.5*x² if |x| ≤ δ, else δ*(|x| - 0.5*δ)
pub struct HuberLoss;

impl HuberLoss {
    pub fn compute(errors: &[f32], delta: f32) -> Result<f32, Box<dyn Error>> {
        if errors.is_empty() {
            return Err("Empty error vector".into());
        }

        let mut total = 0.0f32;
        for &e in errors {
            let abs_e = e.abs();
            let loss = if abs_e <= delta {
                0.5 * e * e
            } else {
                delta * (abs_e - 0.5 * delta)
            };
            total += loss;
        }
        Ok(total / errors.len() as f32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chamfer_identical_points() {
        let points = vec![(0.0, 0.0, 0.0), (1.0, 0.0, 0.0), (0.0, 1.0, 0.0)];
        let loss = ChamferDistance::compute_loss(&points, &points).unwrap();
        assert!(loss < 0.001, "Identical point clouds should have near-zero loss");
    }

    #[test]
    fn test_chamfer_symmetry() {
        let points_a = vec![(0.0, 0.0, 0.0), (1.0, 1.0, 1.0)];
        let points_b = vec![(0.5, 0.5, 0.5), (1.5, 1.5, 1.5)];

        let loss_ab = ChamferDistance::compute_loss(&points_a, &points_b).unwrap();
        let loss_ba = ChamferDistance::compute_loss(&points_b, &points_a).unwrap();

        assert!((loss_ab - loss_ba).abs() < 0.001, "Chamfer distance should be symmetric");
    }

    #[test]
    fn test_chamfer_single_point() {
        let p1 = vec![(0.0, 0.0, 0.0)];
        let p2 = vec![(0.0, 0.0, 0.0)];
        let loss = ChamferDistance::compute_loss(&p1, &p2).unwrap();
        assert_eq!(loss, 0.0);
    }

    #[test]
    fn test_huber_small_errors() {
        let errors = vec![0.1, 0.2, 0.3];
        let loss = HuberLoss::compute(&errors, 1.0).unwrap();
        // 0.5 * (0.01 + 0.04 + 0.09) / 3 ≈ 0.047
        assert!(loss < 0.1, "Small errors should give small Huber loss");
    }

    #[test]
    fn test_huber_large_errors() {
        let errors = vec![10.0];
        let loss = HuberLoss::compute(&errors, 1.0).unwrap();
        // δ * (|e| - 0.5*δ) = 1.0 * (10.0 - 0.5) = 9.5
        assert!((loss - 9.5).abs() < 0.1, "Large errors should use linear Huber loss");
    }

    #[test]
    fn test_chamfer_distance_formula() {
        // Points where distances are known
        let pred = vec![(0.0, 0.0, 0.0)];
        let truth = vec![(1.0, 0.0, 0.0)];
        let loss = ChamferDistance::compute_loss(&pred, &truth).unwrap();
        
        // Distance = 1.0, loss = 1.0²/2 + 1.0²/2 / 2 = 0.5
        assert!((loss - 0.5).abs() < 0.01);
    }
}
