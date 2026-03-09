use twister::computer_vision::pose_estimator::{MockPoseInference, PoseInference};
use twister::ml::body_region_classifier::BodyRegionClassifier;
use twister::ml::pose_materials::{pose_frame_to_point_cloud, PointCloudWithMaterials};
use twister::fusion::imu_pose_fusion::IMUPoseFusion;
use twister::ml::pose_mamba_trainer::{PoseMambaInput, PoseMambaTrainer};
use twister::ml::data_contracts::{IMUSample, PointMambaEncoderOutput, RFDetection};
use std::time::Instant;

#[test]
fn test_track_i_pipeline() {
    let start_time = Instant::now();

    // 1. Pose Inference
    let pose_estimator = MockPoseInference::new();
    let frame = vec![0u8; 100];
    let pose = pose_estimator.estimate_pose(&frame, 256, 256);
    assert_eq!(pose.keypoints.len(), 33);

    // 2. Pose to Materials
    let classifier = BodyRegionClassifier::new();
    let pc: PointCloudWithMaterials = pose_frame_to_point_cloud(&pose, &classifier);
    assert!(pc.points.len() > 0);

    // 3. IMU + Pose Fusion
    let mut fusion = IMUPoseFusion::new();
    let imu = IMUSample {
        accel: [0.0, -9.81, 0.0],
        gyro: [0.1, 0.0, 0.2],
        timestamp: pose.timestamp_us,
    };
    let fused = fusion.fuse(&pose, &imu);
    assert!(fused.gravity_aligned);

    // 4. PointMamba Training
    let mut trainer = PoseMambaTrainer::new();
    let mamba = PointMambaEncoderOutput { embedding: [0.0; 256] };
    let rf = RFDetection {
        azimuth: 0.0,
        elevation: 0.0,
        frequency: 2400.0,
        intensity: 0.8,
        timestamp: pose.timestamp_us,
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

    // Verification
    let duration = start_time.elapsed();
    println!("Pipeline completed in {:?}", duration);
    assert!(duration.as_millis() < 50, "Pipeline took too long!");
}
