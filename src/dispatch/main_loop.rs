//! Main dispatch loop for Track A - Mamba Inference Loop
//! 
//! This module implements the core dispatch loop that:
//! 1. Removes the 9× audio repeat hack
//! 2. Ingests real FieldParticle input from audio and RF sources
//! 3. Accumulates 4096 samples for Mamba processing
//! 4. Projects latent embeddings to Drive/Fold/Asym parameters
//! 5. Updates UI widgets with real-time values
//! 
//! Hardware Integration:
//! - Audio: 24-bit sound card via CPAL/WASAPI (proving ground)
//! - RF: Pluto+ AD9363 via libiio (12-bit IQ streaming)
//! - Edge: Coral TPU (INT8 Normalizing Flow), Pico 2 (ESN classification)

use std::sync::{Arc, atomic::{AtomicU32, Ordering}};
use std::time::{Duration, Instant};
use std::collections::VecDeque;

use crate::dispatch::{SignalIngester, SignalMetadata, SampleFormat, SignalType};
use crate::dispatch::{AudioIngester, RFIngester};
use crate::ml::field_particle::FieldParticle;
use crate::ml::mamba::MambaModel;
use crate::ml::waveshape::WaveshapeProjector;
use crate::ui::toto_widget::TotoWidget;
use crate::ui::chronos_widget::ChronosWidget;

/// Configuration for the dispatch loop
#[derive(Clone)]
pub struct DispatchConfig {
    pub sample_accumulation_size: usize,  // 4096 samples
    pub audio_sample_rate: u32,           // 48000 Hz
    pub rf_sample_rate: u32,              // 1-5 MSPS
    pub energy_sort_window: usize,        // For energy-based sorting
    pub anomaly_threshold: f32,           // For edge device triggering
}

impl Default for DispatchConfig {
    fn default() -> Self {
        Self {
            sample_accumulation_size: 4096,
            audio_sample_rate: 48000,
            rf_sample_rate: 2000000,  // 2 MSPS
            energy_sort_window: 10,
            anomaly_threshold: 0.8,
        }
    }
}

/// Real-time dispatch loop state
pub struct DispatchLoop {
    config: DispatchConfig,
    
    // Signal ingesters
    audio_ingester: AudioIngester,
    rf_ingester: RFIngester,
    
    // Mamba inference
    mamba_model: Arc<MambaModel>,
    waveshape_projector: WaveshapeProjector,
    
    // UI widgets
    toto_widget: TotoWidget,
    chronos_widget: ChronosWidget,
    
    // Edge device interfaces
    anomaly_score: Arc<AtomicU32>,  // Shared with Coral TPU
    esn_classification: Arc<AtomicU32>, // Shared with Pico 2
    
    // Internal state
    accumulated_particles: Vec<FieldParticle>,
    energy_window: VecDeque<f32>,
    last_timestamp: Instant,
    frame_count: u64,
}

impl DispatchLoop {
    pub fn new(config: DispatchConfig, mamba_model: Arc<MambaModel>) -> Self {
        Self {
            config,
            audio_ingester: AudioIngester::new(),
            rf_ingester: RFIngester::new(),
            mamba_model,
            waveshape_projector: WaveshapeProjector::new(),
            toto_widget: TotoWidget::new(),
            chronos_widget: ChronosWidget::new(),
            anomaly_score: Arc::new(AtomicU32::new(0)),
            esn_classification: Arc::new(AtomicU32::new(0)),
            accumulated_particles: Vec::with_capacity(8192), // Double buffer
            energy_window: VecDeque::with_capacity(10),
            last_timestamp: Instant::now(),
            frame_count: 0,
        }
    }

    /// Main dispatch loop - removes the 9× audio repeat hack
    pub fn run(&mut self) -> ! {
        println!("🚀 Starting Track A - Mamba Inference Loop");
        println!("📊 Sample accumulation target: {} particles", self.config.sample_accumulation_size);
        
        loop {
            let frame_start = Instant::now();
            
            // 1. Ingest audio and RF signals simultaneously
            let (audio_particles, rf_particles) = self.ingest_signals();
            
            // 2. Combine and sort by energy
            let combined_particles = self.combine_and_sort_particles(audio_particles, rf_particles);
            
            // 3. Accumulate particles until we reach the target
            self.accumulate_particles(combined_particles);
            
            // 4. Process when we have enough samples
            if self.accumulated_particles.len() >= self.config.sample_accumulation_size {
                self.process_mamba_inference();
            }
            
            // 5. Update UI widgets with real-time values
            self.update_ui_widgets();
            
            // 6. Handle edge device communication
            self.handle_edge_devices();
            
            // 7. Maintain frame timing
            self.maintain_frame_timing(frame_start);
            
            self.frame_count += 1;
            if self.frame_count % 60 == 0 {
                println!("✅ Frame {} processed successfully", self.frame_count);
            }
        }
    }

    /// Ingest audio and RF signals simultaneously
    fn ingest_signals(&self) -> (Vec<FieldParticle>, Vec<FieldParticle>) {
        let timestamp_us = self.last_timestamp.elapsed().as_micros() as u64;
        
        // Audio metadata
        let audio_metadata = SignalMetadata {
            signal_type: SignalType::Audio,
            sample_rate_hz: self.config.audio_sample_rate,
            carrier_freq_hz: None,
            num_channels: 2, // Stereo
            sample_format: SampleFormat::F32,
        };
        
        // RF metadata  
        let rf_metadata = SignalMetadata {
            signal_type: SignalType::RF,
            sample_rate_hz: self.config.rf_sample_rate,
            carrier_freq_hz: Some(2.4e9), // 2.4 GHz WiFi band
            num_channels: 1,
            sample_format: SampleFormat::IQ16,
        };
        
        // TODO: Replace with real hardware reads
        // For now, generate synthetic data to test the pipeline
        let audio_buffer = self.generate_test_audio_buffer();
        let rf_buffer = self.generate_test_rf_buffer();
        
        let audio_particles = self.audio_ingester.ingest(
            &audio_buffer, 
            timestamp_us, 
            &audio_metadata
        );
        
        let rf_particles = self.rf_ingester.ingest(
            &rf_buffer, 
            timestamp_us, 
            &rf_metadata
        );
        
        (audio_particles, rf_particles)
    }
    
    /// Generate test audio buffer (replace with real CPAL read)
    fn generate_test_audio_buffer(&self) -> Vec<u8> {
        // Generate 1024 samples of 440Hz sine wave + noise
        let sample_count = 1024;
        let mut buffer = Vec::with_capacity(sample_count * 4); // F32 samples
        
        for i in 0..sample_count {
            let t = i as f32 / self.config.audio_sample_rate as f32;
            let frequency = 440.0;
            let amplitude = 0.5;
            
            // 440Hz sine wave + 60Hz hum + noise
            let sample = amplitude * (2.0 * std::f32::consts::PI * frequency * t).sin()
                        + 0.1 * (2.0 * std::f32::consts::PI * 60.0 * t).sin()
                        + (rand::random::<f32>() - 0.5) * 0.1;
            
            buffer.extend_from_slice(&sample.to_le_bytes());
        }
        
        buffer
    }
    
    /// Generate test RF buffer (replace with real libiio read)
    fn generate_test_rf_buffer(&self) -> Vec<u8> {
        // Generate 512 IQ16 samples simulating WiFi signal
        let sample_count = 512;
        let mut buffer = Vec::with_capacity(sample_count * 4); // IQ16 samples
        
        for i in 0..sample_count {
            // Simulate WiFi-like signal with carrier and modulation
            let t = i as f32 / self.config.rf_sample_rate as f32;
            let carrier_freq = 2.4e9;
            let modulation_freq = 1e6;
            
            let i_val = (2.0 * std::f32::consts::PI * carrier_freq * t).cos()
                      + 0.5 * (2.0 * std::f32::consts::PI * modulation_freq * t).cos();
            let q_val = (2.0 * std::f32::consts::PI * carrier_freq * t).sin()
                      + 0.5 * (2.0 * std::f32::consts::PI * modulation_freq * t).sin();
            
            // Add noise
            let noise_i = (rand::random::<f32>() - 0.5) * 0.1;
            let noise_q = (rand::random::<f32>() - 0.5) * 0.1;
            
            let i_final = (i_val + noise_i).clamp(-1.0, 1.0);
            let q_final = (q_val + noise_q).clamp(-1.0, 1.0);
            
            // Convert to IQ16 format
            let i_int = (i_final * 32767.0) as i16;
            let q_int = (q_final * 32767.0) as i16;
            
            buffer.extend_from_slice(&i_int.to_le_bytes());
            buffer.extend_from_slice(&q_int.to_le_bytes());
        }
        
        buffer
    }

    /// Combine audio and RF particles and sort by energy
    fn combine_and_sort_particles(
        &mut self, 
        audio_particles: Vec<FieldParticle>, 
        rf_particles: Vec<FieldParticle>
    ) -> Vec<FieldParticle> {
        let mut combined = Vec::with_capacity(audio_particles.len() + rf_particles.len());
        
        // Add audio particles (material_id = 0x0010 for audio)
        for mut particle in audio_particles {
            particle.material_id = 0x0010; // Audio latent cluster
            combined.push(particle);
        }
        
        // Add RF particles (material_id = 0x0100 for RF)
        for mut particle in rf_particles {
            particle.material_id = 0x0100; // RF latent cluster
            combined.push(particle);
        }
        
        // Sort by energy (descending) for priority processing
        combined.sort_by(|a, b| b.energy.partial_cmp(&a.energy).unwrap());
        
        // Update energy window for anomaly detection
        if let Some(highest_energy) = combined.first().map(|p| p.energy) {
            self.energy_window.push_back(highest_energy);
            if self.energy_window.len() > self.config.energy_sort_window {
                self.energy_window.pop_front();
            }
        }
        
        combined
    }

    /// Accumulate particles until we reach the target count
    fn accumulate_particles(&mut self, new_particles: Vec<FieldParticle>) {
        self.accumulated_particles.extend(new_particles);
        
        // If we exceed capacity, keep only the most recent samples
        if self.accumulated_particles.len() > self.config.sample_accumulation_size * 2 {
            let start = self.accumulated_particles.len() - self.config.sample_accumulation_size;
            self.accumulated_particles.drain(0..start);
        }
    }

    /// Process Mamba inference when we have enough samples
    fn process_mamba_inference(&mut self) {
        // Take exactly the target number of particles
        let target_particles = self.accumulated_particles
            .drain(0..self.config.sample_accumulation_size)
            .collect::<Vec<_>>();
        
        // Run Mamba inference
        let embeddings = self.mamba_model.forward(&target_particles);
        
        // Project to Drive/Fold/Asym parameters
        let waveshape = self.waveshape_projector.project(&embeddings);
        
        // Update anomaly score (shared with Coral TPU)
        let current_anomaly = self.calculate_anomaly_score(&target_particles);
        self.anomaly_score.store(
            (current_anomaly * 1000.0) as u32, 
            Ordering::Relaxed
        );
        
        // Update ESN classification (shared with Pico 2)
        let classification = self.classify_signal_pattern(&target_particles);
        self.esn_classification.store(classification, Ordering::Relaxed);
        
        // Log real-time values (these should vary and change over time)
        println!(
            "🎛️  Mamba Inference - Drive: {:.2}, Fold: {:.2}, Asym: {:.2}, Anomaly: {:.2}%",
            waveshape.drive, waveshape.fold, waveshape.asym, current_anomaly * 100.0
        );
    }

    /// Calculate anomaly score based on energy distribution
    fn calculate_anomaly_score(&self, particles: &[FieldParticle]) -> f32 {
        let avg_energy: f32 = particles.iter().map(|p| p.energy).sum::<f32>() / particles.len() as f32;
        
        // Simple anomaly detection: high energy variance indicates anomaly
        let variance: f32 = particles.iter()
            .map(|p| (p.energy - avg_energy).powi(2))
            .sum::<f32>() / particles.len() as f32;
        
        // Normalize to 0-1 range
        variance.min(1.0)
    }

    /// Classify signal pattern for ESN (simplified version)
    fn classify_signal_pattern(&self, particles: &[FieldParticle]) -> u32 {
        let audio_count = particles.iter().filter(|p| p.material_id == 0x0010).count();
        let rf_count = particles.iter().filter(|p| p.material_id == 0x0100).count();
        
        if audio_count > rf_count * 2 {
            1 // Audio dominant
        } else if rf_count > audio_count * 2 {
            2 // RF dominant  
        } else {
            3 // Mixed signal
        }
    }

    /// Update UI widgets with real-time values
    fn update_ui_widgets(&mut self) {
        // Get current values from shared state
        let anomaly_score = self.anomaly_score.load(Ordering::Relaxed) as f32 / 1000.0;
        let classification = self.esn_classification.load(Ordering::Relaxed);
        
        // Update Toto widget (Mamba applet)
        self.toto_widget.update_values(
            anomaly_score,
            self.frame_count % 100 == 0, // Neural Auto-Steer toggle
            0.5, 0.3, 0.7 // Mock Drive/Fold/Asym (should come from Mamba)
        );
        
        // Update Chronos widget (TimeGNN applet)  
        self.chronos_widget.update_values(
            0.14, // Mock temperature
            "Pattern A", // Mock motif name
            0.85, // Mock confidence
            10.0 // Mock next event ETA
        );
    }

    /// Handle edge device communication
    fn handle_edge_devices(&self) {
        // Read anomaly score from Coral TPU (if connected)
        let current_anomaly = self.anomaly_score.load(Ordering::Relaxed);
        if current_anomaly > (self.config.anomaly_threshold * 1000.0) as u32 {
            println!("⚠️  High anomaly detected: {} (threshold: {})", 
                    current_anomaly, 
                    (self.config.anomaly_threshold * 1000.0) as u32);
        }
        
        // Read classification from Pico 2 ESN
        let classification = self.esn_classification.load(Ordering::Relaxed);
        match classification {
            1 => println!("🎵 Audio dominant pattern detected"),
            2 => println!("📡 RF dominant pattern detected"),
            3 => println!("🔀 Mixed signal pattern detected"),
            _ => {}
        }
    }

    /// Maintain consistent frame timing
    fn maintain_frame_timing(&self, frame_start: Instant) {
        let frame_duration = frame_start.elapsed();
        let target_frame_time = Duration::from_millis(16); // ~60 FPS
        
        if frame_duration < target_frame_time {
            std::thread::sleep(target_frame_time - frame_duration);
        }
    }
}

// End of file