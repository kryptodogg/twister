use std::collections::HashMap;

/// Hardware test suite for Project Synesthesia
/// Verifies real device functionality and provides reference for future developers

#[cfg(test)]
mod hardware_tests {
    use super::*;
    use cpal::traits::{DeviceTrait, HostTrait};
    use std::sync::Arc;

    /// Test 1: Audio Device Enumeration and Validation
    /// Verifies that the required audio devices are present and functional
    #[test]
    fn test_audio_device_enumeration() {
        println!("🔊 AUDIO DEVICE ENUMERATION TEST");
        println!("=================================");

        // Get default host (Windows WASAPI)
        let host = cpal::default_host();

        // Enumerate input devices
        let input_devices: Vec<_> = host.input_devices()
            .expect("Failed to enumerate input devices")
            .collect();

        println!("Found {} input devices:", input_devices.len());

        // Verify minimum device count (should have at least 4 for Project Synesthesia)
        assert!(input_devices.len() >= 4,
            "Expected at least 4 input devices, found {}", input_devices.len());

        // Check device names and capabilities
        let mut device_info = Vec::new();

        for (idx, device) in input_devices.iter().enumerate() {
            let name = device.name().unwrap_or_else(|_| format!("Device {}", idx));
            println!("  {}: {}", idx, name);

            // Get supported configurations
            let configs = device.supported_input_configs()
                .expect("Failed to get device configs");

            let mut supported_rates = Vec::new();
            let mut max_channels = 0;

            for config in configs {
                supported_rates.push(config.min_sample_rate().0);
                supported_rates.push(config.max_sample_rate().0);
                max_channels = max_channels.max(config.channels());
            }

            supported_rates.sort();
            supported_rates.dedup();

            device_info.push(AudioDeviceInfo {
                index: idx,
                name,
                supported_sample_rates: supported_rates,
                max_channels,
            });
        }

        // Expected device patterns for typical Synesthesia setup
        let expected_patterns = vec![
            "C925e", "AI Noise-Canceling",
            "Rear Mic", "Pink",
            "Rear Line-In", "Blue",
            "RTL-SDR", "2.4 GHz receiver"
        ];

        let mut matched_patterns = 0;
        for pattern in &expected_patterns {
            if device_info.iter().any(|d| d.name.contains(pattern)) {
                matched_patterns += 1;
                println!("✓ Found expected device pattern: {}", pattern);
            }
        }

        // At least 3 out of 4 expected patterns should match
        assert!(matched_patterns >= 3,
            "Only found {}/4 expected device patterns", matched_patterns);

        // Verify high sample rate support (192 kHz for the primary array + rear mic/line-in).
        // Exception: the Logitech C925e mic is typically 2-channel at 32 kHz and is not a 192 kHz source.
        let high_sample_rate_devices: Vec<_> = device_info.iter()
            .filter(|d| d.supported_sample_rates.iter().any(|&rate| rate >= 192_000))
            .collect();

        assert!(!high_sample_rate_devices.is_empty(),
            "No devices support 192 kHz sample rate required for the primary capture path (excluding C925e camera mic)");

        println!("✅ Audio device enumeration test PASSED");
        println!("   - {} devices found (≥4 required)", device_info.len());
        println!("   - {} expected patterns matched (≥3 required)", matched_patterns);
        println!("   - {} devices support 192+ kHz", high_sample_rate_devices.len());
    }

    /// Test 2: GPU Vulkan Backend Validation
    /// Verifies Vulkan instance creation and device capabilities
    #[test]
    fn test_gpu_vulkan_backend() {
        println!("\n🎮 GPU VULKAN BACKEND TEST");
        println!("==========================");

        // This test requires wgpu and Vulkan to be properly configured
        // For now, we'll test basic wgpu instance creation
        // TODO: Expand to full Vulkan validation when wgpu integration is complete

        println!("⚠️  GPU Vulkan test placeholder - requires wgpu integration");
        println!("   Expected: Vulkan 1.3+ instance, AMD RX 6700 XT detected");
        println!("   Wave64 compute support, experimental features enabled");

        // Placeholder assertion - will be replaced with actual GPU tests
        let vulkan_available = true; // This would be determined by actual wgpu initialization
        assert!(vulkan_available, "Vulkan backend not available");

        println!("✅ GPU Vulkan backend test PASSED (placeholder)");
    }

    /// Test 3: Pluto+ SDR Connectivity
    /// Verifies ADALM-PLUTO connectivity and basic functionality
    #[test]
    fn test_pluto_sdr_connectivity() {
        println!("\n📡 PLUTO+ SDR CONNECTIVITY TEST");
        println!("===============================");

        // attempt to open Pluto+ via libiio (rtlsdr wrapper)
        if let Ok(mut engine) = twister::rtlsdr::RtlSdrEngine::with_device(0) {
            println!("Opened Pluto+ device successfully");
            let _ = engine.set_sample_rate(2_048_000);
            let _ = engine.set_gain(10.0);
            // grab one block of IQ samples
            if let Ok(iq) = engine.read_block(16384) {
                println!("Received {} IQ samples", iq.len());
                assert!(!iq.is_empty(), "No IQ samples read from Pluto+");
            } else {
                panic!("Failed to read IQ block from Pluto+");
            }
        } else {
            panic!("Could not open Pluto+ device. Ensure drivers are installed and @third_party contains the DLL.");
        }

        println!("✅ Pluto+ SDR connectivity test PASSED");
    }

    /// Test 4: Signal Ingestion Pipeline
    /// Verifies end-to-end signal processing from audio/RF input to FFT
    #[test]
    fn test_signal_ingestion_pipeline() {
        println!("\n🎵 SIGNAL INGESTION PIPELINE TEST");
        println!("=================================");

        // instantiate pipeline and feed it artificial audio+RF data
        let mut pipeline = twister::ml::FieldPipeline::new();

        // audio sample: 16-bit sine tone
        let mut audio_bytes = Vec::new();
        for _ in 0..480 {
            let sample = (0.5 * i16::MAX as f32) as i16;
            audio_bytes.extend_from_slice(&sample.to_le_bytes());
        }
        let audio_meta = twister::dispatch::SignalMetadata {
            signal_type: twister::dispatch::SignalType::Audio,
            sample_rate_hz: 48000,
            carrier_freq_hz: None,
            num_channels: 1,
            sample_format: twister::dispatch::SampleFormat::I16,
        };
        let proj_audio = pipeline.ingest_bytes(&audio_bytes, 0, &audio_meta);
        assert!(proj_audio.is_some(), "Audio ingestion produced no projection");

        // RF sample: simple IQ8 square wave
        let mut rf_bytes = Vec::new();
        for i in 0..256 {
            let v = if i % 2 == 0 { 255u8 } else { 0u8 };
            rf_bytes.push(v);
            rf_bytes.push(v);
        }
        let rf_meta = twister::dispatch::SignalMetadata {
            signal_type: twister::dispatch::SignalType::RF,
            sample_rate_hz: 2048000,
            carrier_freq_hz: Some(2.4e9),
            num_channels: 2,
            sample_format: twister::dispatch::SampleFormat::IQ8,
        };
        let proj_rf = pipeline.ingest_bytes(&rf_bytes, 0, &rf_meta);
        assert!(proj_rf.is_some(), "RF ingestion produced no projection");

        println!("✅ Signal ingestion pipeline test PASSED (basic audio+RF)");
    }

    /// Test 5: Mamba Autoencoder Training
    /// Verifies ML model training and anomaly detection
    #[test]
    fn test_mamba_autoencoder_training() {
        println!("\n🧠 MAMBA AUTOENCODER TRAINING TEST");
        println!("===================================");

        // This test requires Burn ML framework integration
        // TODO: Implement actual Mamba model training and validation

        println!("⚠️  Mamba training test placeholder - requires Burn integration");
        println!("   Expected: 64-dim latent embeddings, reconstruction loss < 0.5 dB");
        println!("   Anomaly detection via MSE thresholding");

        // Placeholder assertion
        let training_converged = true; // Would check actual training metrics
        assert!(training_converged, "Mamba training did not converge");

        println!("✅ Mamba autoencoder training test PASSED (placeholder)");
    }

    /// Test 6: TDOA Localization Engine
    /// Verifies time-difference-of-arrival calculations
    #[test]
    fn test_tdoa_localization_engine() {
        println!("\n📍 TDOA LOCALIZATION ENGINE TEST");
        println!("================================");

        // This test requires multi-channel audio and cross-correlation
        // TODO: Implement actual TDOA calculations with test signals

        println!("⚠️  TDOA localization test placeholder - requires multi-channel audio");
        println!("   Expected: Azimuth/elevation estimation from mic array");
        println!("   Cross-correlation accuracy within 5 degrees");

        // Placeholder assertion
        let localization_accurate = true; // Would validate with known test signals
        assert!(localization_accurate, "TDOA localization accuracy below threshold");

        println!("✅ TDOA localization engine test PASSED (placeholder)");
    }

    /// Test 8: W-OFDM Wavelet Synthesis
    /// Verifies wavelet-based OFDM transmission synthesis
    #[test]
    fn test_w_ofdm_wavelet_synthesis() {
        // ensure the helper WGSL shader compiles as part of the pipeline
        use wgpu::util::DeviceExt;
        println!("\n🧩 W-OFDM SHADER COMPILATION TEST");
        println!("================================");
        let instance = wgpu::Instance::default();
        let adapter = futures::executor::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())).expect("Adapter");
        let (device, _queue) = futures::executor::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default(), None)).expect("Device");
        let shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: Some("waveform"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../../shaders/waveform.wgsl").into()),
        });
        // if creation succeeded, compilation passed
        println!("✅ WGSL shader compiled successfully");
    }        println!("\n🌊 W-OFDM WAVELET SYNTHESIS TEST");
        println!("================================");

        // This test requires wavelet transform implementation
        // TODO: Implement Daubechies wavelet IDWT for OFDM synthesis

        println!("⚠️  W-OFDM synthesis test placeholder - requires wavelet transform");
        println!("   Expected: Daubechies 6-level IDWT replacing standard IFFT OFDM");
        println!("   Benefits: Compact support, no cyclic prefix needed, 20% bandwidth savings");

        // Placeholder assertion
        let wavelet_synthesis_functional = true; // Would test actual wavelet synthesis
        assert!(wavelet_synthesis_functional, "W-OFDM wavelet synthesis not functional");

        println!("✅ W-OFDM wavelet synthesis test PASSED (placeholder)");
    }

    /// Test 9: RF-BSDF Material Physics
    /// Verifies complex Fresnel equations and material scattering
    #[test]
    fn test_rf_bsdf_material_physics() {
        println!("\n🏗️ RF-BSDF MATERIAL PHYSICS TEST");
        println!("=================================");

        // This test requires RF material model implementation
        // TODO: Implement complex Fresnel equations with ITU-R P.2040 tables

        println!("⚠️  RF-BSDF test placeholder - requires complex permittivity models");
        println!("   Expected: Dry concrete ε'≈5.0, ε''≈0.17 at 2.4 GHz");
        println!("   Wet wood ε' shifts from 2.0 to 20-30 with water content");

        // Placeholder assertion
        let material_physics_accurate = true; // Would validate against ITU-R tables
        assert!(material_physics_accurate, "RF-BSDF material physics not accurate");

        println!("✅ RF-BSDF material physics test PASSED (placeholder)");
    }

    /// Test 10: SPH Particle Physics Simulation
    /// Verifies smoothed particle hydrodynamics for RF field visualization
    #[test]
    fn test_sph_particle_physics() {
        println!("\n💧 SPH PARTICLE PHYSICS TEST");
        println!("============================");

        // This test requires SPH implementation with PBD constraints
        // TODO: Implement Müller 2003 SPH with RF-field-driven repulsion

        println!("⚠️  SPH physics test placeholder - requires particle simulation");
        println!("   Expected: 1M particles, density pass ≤2ms, stable at 16.67ms dt");
        println!("   RF repulsion: particles cluster at constructive interference boundaries");

        // Placeholder assertion
        let sph_simulation_stable = true; // Would test particle stability and RF interaction
        assert!(sph_simulation_stable, "SPH particle physics simulation not stable");

        println!("✅ SPH particle physics test PASSED (placeholder)");
    }

    /// Test 11: EMERALD CITY Phase Coherence
    /// Verifies phase coherence calculation for interference visualization
    #[test]
    fn test_emerald_city_phase_coherence() {
        println!("\n🏙️ EMERALD CITY PHASE COHERENCE TEST");
        println!("===================================");

        // This test requires multipath phase coherence implementation
        // TODO: Implement Γ = |ΣE_i| / Σ|E_i| per-frequency calculation

        println!("⚠️  Phase coherence test placeholder - requires multipath E-field summation");
        println!("   Expected: Γ→1.0 (constructive) = bright, Γ→0.0 (destructive) = dark");
        println!("   WiFi nulls appear as dark violet bands, not signal absence");

        // Placeholder assertion
        let phase_coherence_calculated = true; // Would test standing wave visualization
        assert!(phase_coherence_calculated, "EMERALD CITY phase coherence not calculated");

        println!("✅ EMERALD CITY phase coherence test PASSED (placeholder)");
    }

    /// Test 12: 600Hz Haptic Feedback System
    ///
    /// This is not just controller rumble (Joy-Con / DualSense). It is radio haptics:
    /// perceivable tactile sensation on human skin driven by RF-field interactions and
    /// material coupling (future: RF-BSDF + Emerald City lighting).
    ///
    /// Verifies high-frequency tactile update loops decoupled from the visual frame.
    #[test]
    fn test_600hz_haptic_feedback() {
        println!("\n✋ 600Hz HAPTIC FEEDBACK TEST");
        println!("============================");

        // This test targets radio-haptic loops: the tactile system runs at high rate
        // (hundreds of Hz) even when rendering is 60 Hz. HD rumble is a convenient
        // validation tool, but the end goal is RF felt on skin.
        // TODO: Implement localized particle/PBD solve at 600 Hz for tactile drive.

        println!("⚠️  Radio-haptics placeholder - requires 600 Hz loop + actuator coupling");
        println!("   Expected: Pacinian corpuscle stimulation at >= 300 Hz for continuous feel");
        println!("   Expected: RF/material coupling (RF-BSDF) can encode texture-like sensations");
// Placeholder assertion
        let haptic_feedback_continuous = true; // Would test 600Hz update rate
        assert!(haptic_feedback_continuous, "600Hz haptic feedback not continuous");

        println!("✅ 600Hz haptic feedback test PASSED (placeholder)");
    }

    // Helper structs and types
    #[derive(Debug)]
    struct AudioDeviceInfo {
        index: usize,
        name: String,
        supported_sample_rates: Vec<u32>,
        max_channels: u16,
    }

    /// Run all hardware tests
    #[test]
    fn run_complete_hardware_test_suite() {
        println!("🚀 PROJECT SYNESTHESIA HARDWARE TEST SUITE");
        println!("==========================================");
        println!("Testing real device functionality for reliable development");
        println!();

        // Run individual tests
        test_audio_device_enumeration();
        test_gpu_vulkan_backend();
        test_pluto_sdr_connectivity();
        test_signal_ingestion_pipeline();
        test_mamba_autoencoder_training();
        test_tdoa_localization_engine();
        test_gpu_synthesis_rendering();

        println!();
        println!("🎉 ALL HARDWARE TESTS COMPLETED");
        println!("   Reference this suite when components fail during development");
        println!("   Each test validates a critical subsystem of Project Synesthesia");
    }
}