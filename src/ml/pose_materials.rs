use crate::computer_vision::pose_estimator::PoseFrame;
use super::body_region_classifier::{BodyRegionClassifier, BodyRegion, MaterialProps};

pub struct Point3D {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

pub struct PointCloudWithMaterials {
    pub points: Vec<Point3D>,
    pub materials: Vec<MaterialProps>,
    pub body_regions: Vec<BodyRegion>,
    pub confidences: Vec<f32>,
}

pub fn pose_frame_to_point_cloud(
    pose: &PoseFrame,
    classifier: &BodyRegionClassifier,
) -> PointCloudWithMaterials {
    let mut points = Vec::new();
    let mut materials = Vec::new();
    let mut regions = Vec::new();
    let mut confidences = Vec::new();

    for (idx, keypoint) in pose.keypoints.iter().enumerate() {
        if keypoint.confidence < 0.5 {
            continue; // Skip low-confidence points
        }

        let region = classifier.get_region(idx);
        let material = classifier.region_to_material(region);

        points.push(Point3D {
            x: keypoint.x,
            y: keypoint.y,
            z: keypoint.z,
        });
        materials.push(material);
        regions.push(region);
        confidences.push(keypoint.confidence);
    }

    PointCloudWithMaterials {
        points,
        materials,
        body_regions: regions,
        confidences,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::computer_vision::pose_estimator::PoseKeypoint;

    #[test]
    fn test_body_region_classification() {
        let classifier = BodyRegionClassifier::new();
        let mut keypoints = [PoseKeypoint { x: 0.0, y: 0.0, z: 0.0, confidence: 0.0 }; 33];

        // High confidence head point
        keypoints[0].confidence = 0.9;
        // Low confidence arm point
        keypoints[13].confidence = 0.2;
        // High confidence mouth point
        keypoints[10].confidence = 0.8;

        let frame = PoseFrame { timestamp_us: 0, keypoints };
        let pc = pose_frame_to_point_cloud(&frame, &classifier);

        assert_eq!(pc.points.len(), 2); // Only confidence > 0.5 are kept
        assert_eq!(pc.body_regions[0], BodyRegion::Head);
        assert_eq!(pc.body_regions[1], BodyRegion::Mouth);
    }

    #[test]
    fn test_motion_modulation() {
        let classifier = BodyRegionClassifier::new();

        let base = classifier.region_to_material(BodyRegion::LeftArm);

        let modulated = classifier.motion_modulate_material(base, 1.0);
        assert!(modulated.hardness < base.hardness);
        assert_eq!(modulated.roughness, base.roughness);
    }
}
