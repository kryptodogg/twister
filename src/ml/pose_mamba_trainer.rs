use crate::ml::data_contracts::{PointMambaEncoderOutput, RFDetection};
use crate::ml::pose_materials::PointCloudWithMaterials;

pub struct PoseMambaInput {
    pub point_cloud_materials: PointCloudWithMaterials,
    pub mamba_embedding: PointMambaEncoderOutput,
    pub rf_detection: RFDetection,
}

pub struct PoseMambaTrainer {
    // For mock purposes, just track how many batches were "trained"
    pub batches_processed: usize,
}

impl PoseMambaTrainer {
    pub fn new() -> Self {
        Self { batches_processed: 0 }
    }

    /// Train Mamba to predict RF pattern from pose + materials
    pub fn train_step(&mut self, batch: &[PoseMambaInput]) -> f32 {
        self.batches_processed += 1;

        let mut loss_sum = 0.0;

        for input in batch {
            // Mock: Predict RF intensity based on max wetness of the materials
            let max_wetness = input.point_cloud_materials.materials.iter()
                .map(|m| m.wetness)
                .fold(0.0f32, |a, b| a.max(b));

            let predicted_rf_intensity = max_wetness * 0.8; // Assume strong RF targets wet areas
            let observed_rf_intensity = input.rf_detection.intensity;

            // MSE
            loss_sum += (predicted_rf_intensity - observed_rf_intensity).powi(2);
        }

        if batch.is_empty() {
            0.0
        } else {
            loss_sum / batch.len() as f32
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ml::body_region_classifier::{BodyRegion, MaterialProps};
    use crate::ml::pose_materials::Point3D;

    #[test]
    fn test_pose_mamba_training_step() {
        let mut trainer = PoseMambaTrainer::new();

        let pc = PointCloudWithMaterials {
            points: vec![Point3D { x: 0.0, y: 0.0, z: 0.0 }],
            materials: vec![MaterialProps { hardness: 0.5, roughness: 0.5, wetness: 0.8 }],
            body_regions: vec![BodyRegion::Mouth],
            confidences: vec![1.0],
        };

        let mamba = PointMambaEncoderOutput { embedding: [0.0; 256] };

        let rf = RFDetection {
            azimuth: 0.0,
            elevation: 0.0,
            frequency: 2400.0,
            intensity: 0.6,
            timestamp: 0,
            confidence: 0.9,
        };

        let input = PoseMambaInput {
            point_cloud_materials: pc,
            mamba_embedding: mamba,
            rf_detection: rf,
        };

        let loss = trainer.train_step(&[input]);
        assert!(loss >= 0.0);
        assert_eq!(trainer.batches_processed, 1);
    }
}
