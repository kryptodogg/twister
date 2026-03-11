//! Test Track A - Mamba Inference Loop
//! 
//! This example demonstrates the complete Track A pipeline:
//! 1. Mamba model inference
//! 2. Waveshape projection
//! 3. Widget updates
//! 4. Dispatch loop simulation

use std::sync::{Arc, atomic::{AtomicU32, Ordering}};
use std::time::Instant;

use twister::ml::mamba::{MambaModel, MambaFactory};
use twister::ml::waveshape::{WaveshapeProjector, WaveshapeFactory};
use twister::ui::toto_widget::{TotoWidget};
use twister::ui::chronos_widget::{ChronosWidget, ChronosWidgetFactory};
use twister::ml::field_particle;

fn main() {
    println!("🚀 Starting Track A - Mamba Inference Loop Test");
    
    // Create models and widgets
    let mamba_model = Arc::new(MambaFactory::create_default());
    let waveshape_projector = WaveshapeProjector::new();
    
    let mut toto_widget = TotoWidget::new();
    let mut chronos_widget = ChronosWidgetFactory::create_default();
    
    // Create shared state
    let anomaly_score = Arc::new(AtomicU32::new(0));
    let event_count = Arc::new(AtomicU32::new(0));
    
    println!("📊 Running simulation...");
    
    // Simulate 10 frames of processing
    for frame in 0..10 {
        // Generate test particles (would come from real hardware)
        let particles = generate_test_particles(frame);
        
        // Run Mamba inference
        let embeddings = mamba_model.forward(&particles);
        
        // Project to waveshape parameters
        let waveshape = waveshape_projector.project(&embeddings);
        
        // Calculate anomaly score
        let anomaly_score_val = calculate_anomaly_score(&particles);
        anomaly_score.store((anomaly_score_val * 1000.0) as u32, Ordering::Relaxed);
        
        // Update widgets
        toto_widget.update_values(
            anomaly_score_val,
            frame % 2 == 0, // Toggle neural auto-steer
            waveshape.drive,
            waveshape.fold,
            waveshape.asym
        );
        
        chronos_widget.update_values(
            (frame as f32) * 0.1, // Temperature
            &format!("Pattern {}", frame % 3),
            (frame as f32) * 0.05 + 0.8, // Confidence
            10.0 - (frame as f32) // ETA
        );
        
        // Print results
        println!("🎬 Frame {} Results:", frame);
        println!("   Drive: {:.2}, Fold: {:.2}, Asym: {:.2}", waveshape.drive, waveshape.fold, waveshape.asym);
        println!("   Anomaly: {:.1}%", anomaly_score_val * 100.0);
        println!("   Temperature: {:.2}, Confidence: {:.1}%", chronos_widget.get_parameters().0, chronos_widget.get_parameters().2 * 100.0);
        println!();
    }
    
    println!("✅ Track A test completed successfully!");
    println!("📊 Final widget states:");
    println!("   Toto - Drive: {:.2}, Fold: {:.2}, Asym: {:.2}", 
             toto_widget.get_parameters().0, toto_widget.get_parameters().1, toto_widget.get_parameters().2);
    println!("   Chronos - Temperature: {:.2}, Confidence: {:.1}%", 
             chronos_widget.get_parameters().0, chronos_widget.get_parameters().2 * 100.0);
}

/// Generate test particles for simulation
fn generate_test_particles(frame: usize) -> Vec<field_particle::FieldParticle> {
    let mut particles = Vec::new();
    
    for i in 0..256 {
        let position = [
            (i as f32 / 256.0) * 2.0 - 1.0, // X: -1 to 1
            (frame as f32 / 10.0) * 2.0 - 1.0, // Y: -1 to 1
            0.0 // Z
        ];
        
        let phase_i = (i as f32 / 256.0) * 2.0 - 1.0;
        let phase_q = ((i + frame) as f32 / 256.0) * 2.0 - 1.0;
        let energy = (i as f32 / 256.0) * 0.5 + 0.5;
        
        let material_id = if i % 2 == 0 { 0x0010 } else { 0x0100 };
        
        particles.push(crate::ml::field_particle::FieldParticle {
            position,
            phase_i,
            phase_q,
            energy,
            material_id,
            _padding: [0; 3],
        });
    }
    
    particles
}

/// Calculate anomaly score from particles
fn calculate_anomaly_score(particles: &[field_particle::FieldParticle]) -> f32 {
    if particles.is_empty() {
        return 0.0;
    }
    
    let avg_energy: f32 = particles.iter().map(|p| p.energy).sum::<f32>() / particles.len() as f32;
    
    // Simple anomaly detection: high energy variance indicates anomaly
    let variance: f32 = particles.iter()
        .map(|p| (p.energy - avg_energy).powi(2))
        .sum::<f32>() / particles.len() as f32;
    
    // Normalize to 0-1 range
    variance.min(1.0)
}