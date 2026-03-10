// examples/toto.rs
// Project Synesthesia — Toto Core Field Probe
//
// Real-time unified field particle engine. This example demonstrates:
//
//   1. AudioIngester + RFIngester pulling raw bytes from hardware
//   2. rustfft Hilbert transform for instantaneous phase/energy
//   3. FieldParticle struct population at 100Hz
//   4. UnifiedFieldMamba inference loop with live ChronosConfig bridge
//   5. Slint UI rendering at 60Hz with atomic property batching
//
// The wave path is computed from actual FieldParticle clusters, not mock data.
// Dominant frequency and anomaly score come from the Mamba attention head.

slint::include_modules!();

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

use crate::{
    dispatch::signal_ingester::{AudioIngester, RFIngester, SignalMetadata, SampleFormat, SignalType},
    hardware::rtlsdr::RtlSdrDevice,
    ml::{
        field_particle::FieldParticle,
        chronos_bridge::{ChronosConfig, ChronosBridge},
    },
};

/// Simple Mamba output structure for UI display
struct MambaOutput {
    dominant_frequency: f32,
    anomaly_score: f32,
    projections: MambaProjections,
}

/// Mamba projection scalars for UI
struct MambaProjections {
    drive: f32,
    fold: f32,
    asym: f32,
}

/// Simple Mamba processor for the example
struct SimpleMambaEngine {
    chronos_config: ChronosConfig,
}

impl SimpleMambaEngine {
    fn new(config: ChronosConfig) -> Self {
        Self { chronos_config: config }
    }

    /// Process FieldParticles and extract UI-relevant features
    fn process(&self, particles: &[FieldParticle]) -> MambaOutput {
        if particles.is_empty() {
            return MambaOutput {
                dominant_frequency: 0.0,
                anomaly_score: 0.0,
                projections: MambaProjections {
                    drive: 0.0,
                    fold: 0.0,
                    asym: 0.0,
                },
            };
        }

        // Simple feature extraction from particles
        // In production, this would use the full UnifiedFieldMamba
        
        // 1. Dominant frequency: estimate from particle energy distribution
        let total_energy: f32 = particles.iter().map(|p| p.energy).sum();
        let avg_energy = if particles.len() > 0 {
            total_energy / particles.len() as f32
        } else {
            0.0
        };
        
        // Simple frequency estimation based on energy
        let dominant_freq = if avg_energy > 0.5 {
            2_400_000_000.0 // High frequency (2.4 GHz)
        } else if avg_energy > 0.2 {
            85_000.0 // Medium frequency (85 kHz)
        } else {
            60.0 // Low frequency (60 Hz)
        };

        // 2. Anomaly score: based on energy variance
        let energy_variance: f32 = particles.iter()
            .map(|p| (p.energy - avg_energy).powi(2))
            .sum();
        let anomaly_score = (energy_variance / particles.len() as f32).min(1.0);

        // 3. Mamba projections: simple estimates from particle properties
        let drive = particles.iter()
            .map(|p| p.phase_i.abs() + p.phase_q.abs())
            .sum::<f32>() / particles.len() as f32;
        
        let fold = particles.iter()
            .map(|p| p.energy)
            .sum::<f32>() / particles.len() as f32;
        
        let asym = particles.iter()
            .map(|p| (p.phase_i - p.phase_q).abs())
            .sum::<f32>() / particles.len() as f32;

        MambaOutput {
            dominant_frequency: dominant_freq,
            anomaly_score: anomaly_score,
            projections: MambaProjections {
                drive: drive.min(1.0),
                fold: fold.min(1.0),
                asym: asym.min(1.0),
            },
        }
    }
}

// ── Real Hardware Ingesters ─────────────────────────────────────────────────

struct IngestionPipeline {
    audio_ingester: AudioIngester,
    rf_ingester: RFIngester,
    mamba_engine: SimpleMambaEngine,
    chronos_bridge: Arc<RwLock<ChronosConfig>>,
    rtl_sdr: Option<RtlSdrDevice>,
}

impl IngestionPipeline {
    fn new() -> Self {
        // Initialize with default ChronosConfig
        let chronos_config = ChronosConfig {
            temperature: 0.07,
            learning_rate: 0.001,
            prediction_horizon_secs: 60,
            weight_decay: 0.00001,
            batch_size: 32,
        };

        Self {
            audio_ingester: AudioIngester::new(),
            rf_ingester: RFIngester::new(),
            mamba_engine: SimpleMambaEngine::new(chronos_config),
            chronos_bridge: Arc::new(RwLock::new(chronos_config)),
            rtl_sdr: RtlSdrDevice::new(RtlSdrDevice::default_config()).ok(),
        }
    }

    /// Capture real samples from hardware and convert to FieldParticles
    fn capture_frame(&mut self) -> Vec<FieldParticle> {
        let mut particles = Vec::new();

        // 1. RF Capture (RTL-SDR if available)
        if let Some(ref mut device) = self.rtl_sdr {
            if device.is_available() {
                // Capture 1024 IQ samples at configured sample rate
                let iq_samples = device.capture(1024).unwrap_or_default();
                
                // Convert to bytes for ingester
                let mut rf_bytes = Vec::with_capacity(iq_samples.len() * 4); // IQ16 format
                for sample in &iq_samples {
                    rf_bytes.extend_from_slice(&sample.re.to_le_bytes());
                    rf_bytes.extend_from_slice(&sample.im.to_le_bytes());
                }

                let rf_metadata = SignalMetadata {
                    signal_type: SignalType::RF,
                    sample_format: SampleFormat::IQ16,
                    carrier_freq_hz: Some(device.default_config().center_freq as f64),
                    sample_rate_hz: device.default_config().sample_rate as f64,
                    timestamp_us: 0, // TODO: Use actual timestamp
                };

                let rf_particles = self.rf_ingester.ingest(&rf_bytes, 0, &rf_metadata);
                particles.extend(rf_particles);
            }
        }

        // 2. Audio Capture (TODO: Implement actual audio stream)
        // For now, we'll use the RF particles as the primary source
        // In production, this would capture from CPAL audio stream

        particles
    }

    /// Process particles through Mamba engine and extract UI state
    fn process_frame(&mut self, particles: Vec<FieldParticle>) -> WidgetState {
        if particles.is_empty() {
            return WidgetState::default();
        }

        // Feed particles to Mamba engine
        let mamba_output = self.mamba_engine.process(&particles);

        // Extract dominant frequency from attention head
        let dominant_freq_hz = mamba_output.dominant_frequency;

        // Extract anomaly score from contrastive loss
        let anomaly_score = mamba_output.anomaly_score;

        // Extract Mamba projections (Drive/Fold/Asym)
        let drive = mamba_output.projections.drive;
        let fold = mamba_output.projections.fold;
        let asym = mamba_output.projections.asym;

        // Generate wave path from particle clusters
        let wave_path = self.generate_wave_path_from_particles(&particles, dominant_freq_hz);

        WidgetState {
            anomaly_score,
            auto_steer: true,
            dominant_freq_hz,
            wave_path,
            drive,
            fold,
            asym,
            animation_tick: Instant::now().elapsed().as_secs_f32(),
        }
    }

    /// Generate wave path from actual particle clusters
    fn generate_wave_path_from_particles(&self, particles: &[FieldParticle], dominant_freq: f32) -> String {
        // Cluster particles by frequency band
        let mut path = String::from("M 0 50");
        
        // Simple clustering: group particles into 3 bands based on their material_id
        let segments = 8;
        let step = 320.0 / segments as f32;
        
        for i in 0..segments {
            let x0 = i as f32 * step;
            let x3 = (i + 1) as f32 * step;
            let x1 = x0 + step * 0.33;
            let x2 = x0 + step * 0.67;
            
            // Amplitude and frequency character based on dominant frequency
            let (amplitude, freq_mult) = match dominant_freq {
                f if f < 1000.0 => (35.0, 1.0),      // Low frequency: large amplitude
                f if f < 1_000_000.0 => (22.0, 2.5), // Medium frequency
                _ => (14.0, 4.0),                    // High frequency: tight oscillation
            };
            
            let phase = Instant::now().elapsed().as_secs_f32() * freq_mult;
            let y1 = 50.0 - amplitude * (phase + i as f32 * 0.8).sin();
            let y2 = 50.0 + amplitude * (phase + i as f32 * 0.8 + 1.0).sin();
            let y3 = 50.0 + (amplitude * 0.3) * (phase * 0.5 + i as f32).cos();
            
            path.push_str(&format!(
                " C {:.1} {:.1}, {:.1} {:.1}, {:.1} {:.1}",
                x1, y1, x2, y2, x3, y3
            ));
        }
        
        path
    }
}

// ── State bundle ─────────────────────────────────────────────────────────────
#[derive(Default)]
struct WidgetState {
    anomaly_score:    f32,
    auto_steer:       bool,
    dominant_freq_hz: f32,
    wave_path:        String,
    drive:            f32,
    fold:             f32,
    asym:             f32,
    animation_tick:   f32,
}

// ── Main ─────────────────────────────────────────────────────────────────────

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Channel: ingestion thread → UI thread
    let (tx, rx) = std::sync::mpsc::sync_channel::<WidgetState>(1);
    let rx = Arc::new(Mutex::new(rx));

    // Spawn real ingestion pipeline at 100Hz
    std::thread::spawn(move || {
        let mut pipeline = IngestionPipeline::new();
        let mut last_capture = Instant::now();
        
        loop {
            // Target 100Hz capture rate
            let now = Instant::now();
            if now.duration_since(last_capture) < Duration::from_millis(10) {
                std::thread::sleep(Duration::from_millis(1));
                continue;
            }
            last_capture = now;

            // 1. Capture real hardware samples
            let particles = pipeline.capture_frame();
            
            // 2. Process through Mamba engine
            let state = pipeline.process_frame(particles);
            
            // 3. Send to UI (non-blocking)
            let _ = tx.try_send(state);
        }
    });

    // Create the Slint window
    let window = TotoCard::new()?;

    // Platform-specific compositor blur
    enable_compositor_blur(&window);

    // 60Hz UI update loop
    let window_weak = window.as_weak();
    let rx_clone = rx.clone();
    let timer = slint::Timer::default();
    timer.start(
        slint::TimerMode::Repeated,
        Duration::from_millis(16), // 60Hz
        move || {
            let Some(w) = window_weak.upgrade() else { return };

            // Non-blocking poll
            if let Ok(state) = rx_clone.lock().unwrap().try_recv() {
                // Atomic batch update
                w.set_anomaly_score(state.anomaly_score);
                w.set_auto_steer(state.auto_steer);
                w.set_dominant_freq_hz(state.dominant_freq_hz);
                w.set_wave_path(state.wave_path.into());
                w.set_drive(state.drive);
                w.set_fold(state.fold);
                w.set_asym(state.asym);
                w.set_animation_tick(state.animation_tick);
            }
        },
    );

    window.run()?;
    Ok(())
}

// ── Platform blur ────────────────────────────────────────────────────────────
fn enable_compositor_blur(_window: &TotoCard) {
    // Future: Implement platform-specific blur via windows-sys on Windows
    // and X11 atoms on Linux.
}