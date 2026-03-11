pub mod stream_packer;
pub use stream_packer::GpuStreamPacker;

pub mod signal_ingester;
pub use signal_ingester::{SignalIngester, SignalMetadata, SignalType, SampleFormat};

pub mod audio_ingester;
pub use audio_ingester::AudioIngester;

pub mod rf_ingester;
pub use rf_ingester::RFIngester;

pub mod visual_ingester;
pub use visual_ingester::VisualIngester;

pub mod backend;
pub mod het_synthesizer;

/// Forensic Helper: Generate Density Sparkle Path (Slint SVG)
pub fn generate_density_sparkle(particles: &[crate::ml::field_particle::FieldParticle]) -> String {
    let mut path = String::new();
    for p in particles.iter().take(64) {
        let x = (p.position[0] * 320.0).clamp(0.0, 320.0);
        let y = (p.position[1] * 180.0).clamp(0.0, 180.0);
        path.push_str(&format!("M {:.1} {:.1} L {:.1} {:.1} ", x, y, x + 1.0, y + 1.0));
    }
    path
}
