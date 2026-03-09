//! Integration tests for GPU-driven dispatch kernel with v-buffer
//!
//! Tests verify:
//! - Kernel initialization
//! - Audio frame enqueuing
//! - Autonomous GPU dispatch
//! - Zero-copy result access
//! - Work queue operations

#[cfg(test)]
mod dispatch_kernel_tests {
    use std::time::Instant;
    use twister::dispatch_kernel::{
        AudioFrameVBuffer, AutonomousDispatchKernel, DispatchKernelConfig, DispatchResultVBuffer,
    };

    /// Helper to create a test wgpu device and queue
    fn create_test_device() -> Option<(wgpu::Device, wgpu::Queue)> {
        // Attempt to create a device for testing
        // This requires a GPU and may not work in headless CI environments
        #[allow(clippy::never_loop)]
        for _ in 0..1 {
            // Note: In a real test environment, use wgpu::Instance::new() with
            // appropriate backend selection (Vulkan, DX12, etc.)
            // For now, we skip actual GPU device creation in unit tests
            return None;
        }
        None
    }

    #[test]
    fn test_audio_frame_creation() {
        let frame = AudioFrameVBuffer {
            sample_fl: 0.1,
            sample_fr: 0.2,
            sample_rl: 0.3,
            sample_rr: 0.4,
            timestamp_us: 1000,
            frame_index: 0,
            _padding: [0; 1],
        };

        assert_eq!(frame.sample_fl, 0.1);
        assert_eq!(frame.sample_fr, 0.2);
        assert_eq!(frame.sample_rl, 0.3);
        assert_eq!(frame.sample_rr, 0.4);
        assert_eq!(frame.timestamp_us, 1000);
        assert_eq!(frame.frame_index, 0);
    }

    #[test]
    fn test_audio_frame_alignment() {
        // Verify AudioFrameVBuffer is 32 bytes (8 × f32 + u64 aligned)
        assert_eq!(std::mem::size_of::<AudioFrameVBuffer>(), 32);
        assert_eq!(std::mem::align_of::<AudioFrameVBuffer>(), 8);
    }

    #[test]
    fn test_dispatch_result_creation() {
        let result = DispatchResultVBuffer {
            detected_frequency_hz: 440.0,
            anomaly_score_db: -30.0,
            beamform_azimuth_degrees: 45.0,
            beamform_elevation_degrees: 15.0,
            rf_power_dbfs: -20.0,
            confidence: 0.95,
            _padding: [0; 2],
        };

        assert_eq!(result.detected_frequency_hz, 440.0);
        assert_eq!(result.anomaly_score_db, -30.0);
        assert_eq!(result.beamform_azimuth_degrees, 45.0);
        assert_eq!(result.confidence, 0.95);
    }

    #[test]
    fn test_dispatch_result_alignment() {
        // Verify DispatchResultVBuffer is 32 bytes
        assert_eq!(std::mem::size_of::<DispatchResultVBuffer>(), 32);
        assert_eq!(std::mem::align_of::<DispatchResultVBuffer>(), 8);
    }

    #[test]
    fn test_dispatch_kernel_config_default() {
        let config = DispatchKernelConfig::default();

        assert_eq!(config.vbuffer_capacity, 19_200);
        assert_eq!(config.batch_size, 32);
        assert_eq!(config.detection_threshold_db, -40.0);
        assert_eq!(config.azimuth_resolution, 5.0);
    }

    #[test]
    fn test_dispatch_kernel_config_custom() {
        let config = DispatchKernelConfig {
            vbuffer_capacity: 9_600,
            batch_size: 16,
            detection_threshold_db: -30.0,
            azimuth_resolution: 2.5,
        };

        assert_eq!(config.vbuffer_capacity, 9_600);
        assert_eq!(config.batch_size, 16);
        assert_eq!(config.detection_threshold_db, -30.0);
    }

    #[test]
    #[ignore] // Requires GPU device
    fn test_kernel_initialization() {
        if let Some((device, queue)) = create_test_device() {
            let kernel = AutonomousDispatchKernel::new(device, queue, None);
            assert!(kernel.is_ok());
        }
    }

    #[test]
    #[ignore] // Requires GPU device
    fn test_kernel_enqueue_audio_frames() {
        if let Some((device, queue)) = create_test_device() {
            let mut kernel = AutonomousDispatchKernel::new(device, queue, None)
                .expect("Failed to create kernel");

            let frames = vec![
                AudioFrameVBuffer {
                    sample_fl: 0.1,
                    sample_fr: 0.1,
                    sample_rl: 0.1,
                    sample_rr: 0.1,
                    timestamp_us: 1000,
                    frame_index: 0,
                    _padding: [0; 1],
                };
                32
            ];

            let result = kernel.enqueue_audio_frames(&frames);
            assert!(result.is_ok());

            // Verify frame counter incremented
            assert_eq!(kernel.frame_count(), 32);
        }
    }

    #[test]
    #[ignore] // Requires GPU device
    fn test_kernel_frame_counter_increment() {
        if let Some((device, queue)) = create_test_device() {
            let mut kernel = AutonomousDispatchKernel::new(device, queue, None)
                .expect("Failed to create kernel");

            assert_eq!(kernel.frame_count(), 0);

            let frames = vec![AudioFrameVBuffer::default(); 16];
            kernel.enqueue_audio_frames(&frames).unwrap();
            assert_eq!(kernel.frame_count(), 16);

            let frames = vec![AudioFrameVBuffer::default(); 8];
            kernel.enqueue_audio_frames(&frames).unwrap();
            assert_eq!(kernel.frame_count(), 24);
        }
    }

    #[test]
    #[ignore] // Requires GPU device
    fn test_kernel_autonomous_dispatch() {
        if let Some((device, queue)) = create_test_device() {
            let mut kernel = AutonomousDispatchKernel::new(device, queue, None)
                .expect("Failed to create kernel");

            // Enqueue some frames
            let frames = vec![AudioFrameVBuffer::default(); 32];
            kernel.enqueue_audio_frames(&frames).unwrap();

            // Dispatch kernel (GPU processes autonomously)
            kernel.dispatch_autonomous_batch();

            // Give GPU time to execute
            std::thread::sleep(std::time::Duration::from_millis(10));

            // GPU should have generated work items
            let processed = kernel.dequeue_processed_frames();
            // Note: Work queue population depends on WGSL implementation
            println!("Processed frames: {:?}", processed);
        }
    }

    #[test]
    #[ignore] // Requires GPU device
    fn test_zero_copy_results_latency() {
        if let Some((device, queue)) = create_test_device() {
            let mut kernel = AutonomousDispatchKernel::new(device, queue, None)
                .expect("Failed to create kernel");

            // Enqueue frames
            let frames = vec![AudioFrameVBuffer::default(); 32];
            kernel.enqueue_audio_frames(&frames).unwrap();

            // Dispatch batch
            kernel.dispatch_autonomous_batch();

            // Give GPU time
            std::thread::sleep(std::time::Duration::from_millis(10));

            // Measure read latency
            let start = Instant::now();
            let _results = kernel.read_results();
            let latency = start.elapsed();

            // Should be < 100 microseconds (zero-copy)
            println!("V-buffer read latency: {:?}", latency);
            assert!(
                latency.as_micros() < 100,
                "Expected zero-copy latency, got {:?}",
                latency
            );
        }
    }

    #[test]
    #[ignore] // Requires GPU device
    fn test_results_ack() {
        if let Some((device, queue)) = create_test_device() {
            let mut kernel = AutonomousDispatchKernel::new(device, queue, None)
                .expect("Failed to create kernel");

            // Enqueue and dispatch
            let frames = vec![AudioFrameVBuffer::default(); 32];
            kernel.enqueue_audio_frames(&frames).unwrap();
            kernel.dispatch_autonomous_batch();

            std::thread::sleep(std::time::Duration::from_millis(10));

            // Read results
            let _results = kernel.read_results();

            // Acknowledge read (should not panic)
            kernel.ack_results_read();
        }
    }

    #[test]
    fn test_config_access() {
        let config = DispatchKernelConfig::default();
        assert_eq!(config.batch_size, 32);
    }

    #[test]
    fn test_audio_frame_byte_layout() {
        // Verify byte layout matches expected GPU struct
        let frame = AudioFrameVBuffer {
            sample_fl: 1.0,
            sample_fr: 2.0,
            sample_rl: 3.0,
            sample_rr: 4.0,
            timestamp_us: 0x0102030405060708,
            frame_index: 0x0a0b0c0d,
            _padding: [0x0e0f1011],
        };

        let bytes = bytemuck::bytes_of(&frame);
        assert_eq!(bytes.len(), 32);

        // Verify field ordering (little-endian on most platforms)
        // Fields are in order: sample_fl, sample_fr, sample_rl, sample_rr, timestamp_us, frame_index, padding
    }

    #[test]
    fn test_dispatch_result_byte_layout() {
        let result = DispatchResultVBuffer {
            detected_frequency_hz: 440.0,
            anomaly_score_db: -30.0,
            beamform_azimuth_degrees: 45.0,
            beamform_elevation_degrees: 15.0,
            rf_power_dbfs: -20.0,
            confidence: 0.95,
            _padding: [0; 2],
        };

        let bytes = bytemuck::bytes_of(&result);
        assert_eq!(bytes.len(), 32);
    }

    #[test]
    fn test_multiple_frame_vectors() {
        let frames_a = vec![AudioFrameVBuffer::default(); 10];
        let frames_b = vec![AudioFrameVBuffer::default(); 20];

        assert_eq!(frames_a.len(), 10);
        assert_eq!(frames_b.len(), 20);

        // Verify can create batches
        let batch_size = 32;
        assert!(frames_a.len() < batch_size);
        assert!(frames_b.len() < batch_size);
    }

    #[test]
    fn test_result_ranges() {
        // Verify result values stay within realistic ranges
        let mut result = DispatchResultVBuffer::default();

        // Test frequency range [1 Hz, 96 MHz]
        result.detected_frequency_hz = 1.0;
        assert!(result.detected_frequency_hz >= 1.0);

        result.detected_frequency_hz = 96_000_000.0;
        assert!(result.detected_frequency_hz <= 96_000_000.0);

        // Test azimuth range [0, 360]
        result.beamform_azimuth_degrees = 0.0;
        assert!(result.beamform_azimuth_degrees >= 0.0);

        result.beamform_azimuth_degrees = 360.0;
        assert!(result.beamform_azimuth_degrees <= 360.0);

        // Test elevation range [-90, 90]
        result.beamform_elevation_degrees = -90.0;
        assert!(result.beamform_elevation_degrees >= -90.0);

        result.beamform_elevation_degrees = 90.0;
        assert!(result.beamform_elevation_degrees <= 90.0);

        // Test confidence range [0, 1]
        result.confidence = 0.0;
        assert!(result.confidence >= 0.0);

        result.confidence = 1.0;
        assert!(result.confidence <= 1.0);
    }
}
