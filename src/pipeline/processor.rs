//! Pipeline processor - main processing loop coordinator

use crate::hardware::{RtlSdrCapture, MultiDeviceCapture, RtlSdrConfig, AudioConfig};
use crate::dsp::{FeatureExtractor, FeatureVector};
use crate::mamba::{SSAMBAConfig, MambaInference};
use crate::forensics::{ForensicEvent, RFContext, AudioContext};
use crate::forensics::event::{ControlState, SystemState, EventType, EventMetadata};
use crate::control::{ControlPolicy, PolicyConfig, ControlOutput};
use crate::utils::metrics::{MetricsCollector, LatencyMonitor};
use crate::utils::error::Result;
use ndarray::Array1;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use parking_lot::Mutex;

/// Processor configuration
#[derive(Debug, Clone)]
pub struct ProcessorConfig {
    /// Target latency (ms)
    pub target_latency_ms: u32,
    /// Buffer size (samples)
    pub buffer_size: usize,
    /// Enable forensics logging
    pub enable_forensics: bool,
    /// Enable metrics collection
    pub enable_metrics: bool,
}

impl Default for ProcessorConfig {
    fn default() -> Self {
        Self {
            target_latency_ms: 35,
            buffer_size: 1024,
            enable_forensics: true,
            enable_metrics: true,
        }
    }
}

/// Pipeline processor state
#[derive(Debug, Clone)]
pub struct ProcessorState {
    /// Is pipeline running
    pub running: bool,
    /// Current mode
    pub mode: String,
    /// Current SNR (dB)
    pub snr_db: f32,
    /// Pipeline latency (ms)
    pub latency_ms: f32,
    /// Uptime (seconds)
    pub uptime_secs: u64,
    /// Events processed
    pub events_processed: u64,
}

/// Main pipeline processor
pub struct PipelineProcessor {
    config: ProcessorConfig,
    state: Arc<Mutex<ProcessorState>>,
    metrics: MetricsCollector,
    start_time: Instant,
    events_processed: u64,
}

impl PipelineProcessor {
    /// Create a new pipeline processor
    pub fn new(config: ProcessorConfig) -> Self {
        Self {
            config,
            state: Arc::new(Mutex::new(ProcessorState {
                running: false,
                mode: "silence".into(),
                snr_db: 0.0,
                latency_ms: 0.0,
                uptime_secs: 0,
                events_processed: 0,
            })),
            metrics: MetricsCollector::new(),
            start_time: Instant::now(),
            events_processed: 0,
        }
    }

    /// Run the main processing loop
    pub async fn run(&mut self) -> Result<()> {
        tracing::info!("Starting pipeline processor");
        
        self.state.lock().running = true;

        // Initialize hardware
        let rtlsdr_config = RtlSdrConfig::default();
        let mut rtlsdr = RtlSdrCapture::new(rtlsdr_config)?;
        
        let audio_config = AudioConfig::default();
        let mut audio = MultiDeviceCapture::new(
            audio_config.clone(),
            audio_config.clone(),
        )?;

        // Initialize DSP
        let feature_extractor = FeatureExtractor::new(
            rtlsdr_config.sample_rate,
            audio_config.sample_rate,
        );

        // Initialize ML
        let mamba_config = SSAMBAConfig::new();
        let mamba_inference = MambaInference::new(&mamba_config);

        // Initialize control
        let policy_config = PolicyConfig::default();
        let mut control_policy = ControlPolicy::new(policy_config);

        // Start hardware
        rtlsdr.start().await?;
        audio.start_all().await?;

        tracing::info!("Pipeline started");

        // Main processing loop
        let mut frame_count = 0u64;
        
        while self.state.lock().running {
            let frame_start = Instant::now();

            // Capture RF samples
            let iq_buffer = rtlsdr.device().get_iq_buffer();

            // Capture audio samples
            let audio_pair = audio.get_tdoa_pair();
            let audio_channels = if let Some(ref pair) = audio_pair {
                vec![
                    pair.left_channel.clone(),
                    pair.right_channel.clone(),
                    pair.left_channel.clone(), // Placeholder for 3rd channel
                ]
            } else {
                vec![vec![0.0f32; 1024]; 3]
            };

            // Extract features
            let features = feature_extractor.extract_features_default(
                &iq_buffer,
                &audio_channels,
            );

            // ML inference
            let inference_result = mamba_inference.infer(&features);

            // Control policy
            let audio_flat: Vec<f32> = audio_channels.iter().flatten().copied().collect();
            let control_output = control_policy.update(
                &audio_flat,
                0.3, // RF stress placeholder
                inference_result.mode_probs,
            );

            // Update state
            {
                let mut state = self.state.lock();
                state.mode = format!("{:?}", control_output.mode);
                state.snr_db = control_output.snr_estimate.as_ref().map(|s| s.snr_db).unwrap_or(0.0);
                state.latency_ms = frame_start.elapsed().as_secs_f32() * 1000.0;
                state.uptime_secs = self.start_time.elapsed().as_secs();
                state.events_processed = self.events_processed;
            }

            // Record metrics
            self.metrics.latency.record(frame_start.elapsed());
            self.metrics.rf_capture_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            self.metrics.audio_capture_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            self.metrics.mamba_inference_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            self.metrics.control_update_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

            // Log forensics event (periodically)
            if self.config.enable_forensics && frame_count % 100 == 0 {
                let event = self.create_forensic_event(
                    &features,
                    &inference_result,
                    &control_output,
                );
                // In production, would send to Qdrant/Neo4j
                tracing::debug!("Forensic event: {}", event.id);
            }

            // Check latency budget
            let elapsed = frame_start.elapsed();
            let target = Duration::from_millis(self.config.target_latency_ms as u64);
            if elapsed > target {
                tracing::warn!("Frame latency exceeded: {:?} > {:?}", elapsed, target);
            }

            self.events_processed += 1;
            frame_count += 1;

            // Yield to prevent starvation
            tokio::task::yield_now().await;
        }

        // Cleanup
        rtlsdr.stop().await?;
        audio.stop_all().await?;

        tracing::info!("Pipeline processor stopped");
        Ok(())
    }

    /// Create a forensic event
    fn create_forensic_event(
        &self,
        features: &FeatureVector,
        inference: &crate::mamba::inference::InferenceResult,
        control: &ControlOutput,
    ) -> ForensicEvent {
        use crate::forensics::event::ControlMode as ForensicControlMode;
        
        let control_mode = match control.mode {
            crate::forensics::event::ControlMode::Anc => ForensicControlMode::Anc,
            crate::forensics::event::ControlMode::Silence => ForensicControlMode::Silence,
            crate::forensics::event::ControlMode::Music => ForensicControlMode::Music,
        };

        ForensicEvent::new(
            EventMetadata {
                event_type: EventType::Snapshot,
                location_id: None,
                session_id: "pipeline".into(),
                sequence: self.events_processed,
                tags: vec!["pipeline".into()],
            },
            RFContext {
                center_frequency_hz: 100_000_000,
                sample_rate_hz: 2_048_000,
                psd: features.rf_psd.to_vec(),
                total_power_db: -50.0,
                spectral_kurtosis: 0.0,
                peak_bin: 0,
                band_ratios: [0.33, 0.33, 0.34],
                rfi_detected: false,
                snr_db: 50.0,
            },
            AudioContext {
                sample_rate_hz: 192_000,
                num_channels: 3,
                psd: features.audio_psd.to_vec(),
                tdoa: features.tdoa.to_vec(),
                tdoa_estimate: 0.0,
                correlation_peak: 0.0,
                residual_energy: 0.0,
                channel_energies: [0.0, 0.0, 0.0],
                spectral_centroid: 0.0,
                zcr: 0.0,
                ambient_noise_db: 40.0,
            },
            inference.latent.clone(),
            ControlState {
                mode: control_mode,
                mode_probs: inference.mode_probs,
                target_snr_db: control.target_snr_db,
                anc_weights_version: 0,
                fade_state: control.fade_position,
            },
            SystemState {
                cpu_usage: 0.0,
                memory_mb: 0,
                pipeline_latency_ms: control.snr_estimate.as_ref().map(|s| s.snr_db).unwrap_or(0.0),
                gpu_utilization: 0.0,
                temperature_c: None,
                uptime_secs: self.start_time.elapsed().as_secs(),
            },
        )
    }

    /// Stop the pipeline
    pub fn stop(&mut self) {
        self.state.lock().running = false;
    }

    /// Get current state
    pub fn state(&self) -> ProcessorState {
        self.state.lock().clone()
    }

    /// Get metrics
    pub fn metrics(&self) -> &MetricsCollector {
        &self.metrics
    }

    /// Get uptime
    pub fn uptime(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Get events processed count
    pub fn events_processed(&self) -> u64 {
        self.events_processed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_processor_creation() {
        let config = ProcessorConfig::default();
        let processor = PipelineProcessor::new(config);

        let state = processor.state();
        assert!(!state.running);
        assert_eq!(state.events_processed, 0);
    }

    #[test]
    fn test_processor_state() {
        let config = ProcessorConfig::default();
        let processor = PipelineProcessor::new(config);

        assert_eq!(processor.events_processed(), 0);
        assert!(processor.uptime().as_millis() >= 0);
    }

    #[tokio::test]
    async fn test_processor_stop() {
        let config = ProcessorConfig::default();
        let mut processor = PipelineProcessor::new(config);

        processor.stop();
        
        let state = processor.state();
        assert!(!state.running);
    }
}
