// src/dispatch/mod.rs — IQ Sample Dispatch Pipeline
//
// Tokio-based dispatch loop for streaming IQ samples from RTL-SDR devices to GPU.
// Zero-copy architecture: raw [u8; 2] IQ bytes → DMA → GPU FFT → spectral history.

pub mod iq_dispatch;
pub mod signal_dispatch;
pub mod signal_metadata;

pub use iq_dispatch::IqDispatchLoop;
pub use signal_dispatch::SignalDispatchLoop;
pub use signal_metadata::{MultiRateSignalFrame, SampleDeltaTime, TaggedSignalBuffer};

use std::sync::Arc;

pub fn rt_store_async(
    qdrant: Arc<Option<crate::embeddings::EmbeddingStore>>,
    neo4j: Arc<tokio::sync::Mutex<Option<crate::graph::ForensicGraph>>>,
    event: crate::detection::DetectionEvent,
    state: Arc<crate::state::AppState>,
) {
    state.log(
        "INFO",
        "Forensic",
        &format!(
            "Auto-capture: Detection at {:.1} Hz (Magnitude: {:.2})",
            event.f1_hz, event.magnitude
        ),
    );

    if let Some(store) = (*qdrant).clone() {
        let ev = event.clone();
        tokio::spawn(async move {
            let _ = store.store_detection(&ev).await;
        });
    }

    let n = neo4j.clone();
    let ev = event.clone();
    tokio::spawn(async move {
        if let Some(g) = n.lock().await.as_ref() {
            let _ = g.store_detection(&ev).await;
        }
    });
}
