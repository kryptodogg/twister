//! Dispatch Loop — V3 Track A Mamba Inference
//!
//! # V3 Architecture Notes
//! - Track A1: Dispatch loop stubbed until Track 0-D hardware applet complete
//! - All references to deleted types removed per AGENTS.md §2.5

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
// pub mod het_synthesizer; — deleted, V3 rewrite

use std::sync::Arc;
use crate::state::AppState;
use crate::utils::latency::QpcTimer;

/// Primary Forensic Dispatch Loop
/// 
/// # Track A1 Status
/// Stubbed until Track 0-D hardware applet is complete.
/// Per AGENTS.md §4.3: "The Dispatch Loop Is a Stub Until Track A1"
pub async fn start_dispatch_loop(
    _state: Arc<AppState>,
    _timer: Arc<QpcTimer>,
) {
    todo!("Track A1 — dispatch loop not yet implemented. See SYNESTHESIA_MASTERPLAN.md Track 0-D prerequisites.");
}

pub fn generate_density_sparkle(particles: &[crate::ml::field_particle::FieldParticle]) -> String {
    let mut path = String::new();
    for p in particles.iter().take(64) {
        let x = (p.position[0] * 320.0).clamp(0.0, 320.0);
        let y = (p.position[1] * 180.0).clamp(0.0, 180.0);
        path.push_str(&format!("M {:.1} {:.1} L {:.1} {:.1} ", x, y, x + 1.0, y + 1.0));
    }
    if path.is_empty() { "M 0 0".to_string() } else { path }
}
