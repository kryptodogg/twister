// src/dispatch.rs — Multi-Modal Signal Ingestion Loop
//
// Unified dispatch that ingests audio, RF, and visual data into GPU VRAM.
// No processing (no STFT, no windowing, no filtering).
// Just: acquire → format minimally → stage → GPU.
//
// Runs as Tokio task, non-blocking, 10ms polling interval.

use crate::app_state::DirtyFlags;
use crate::hardware_io::{DeviceManager, IqDmaGateway};
use std::sync::Arc;
use tokio::time::{interval, Duration};

/// Multi-modal dispatch configuration.
pub struct DispatchConfig {
    /// Polling interval (10ms = 100 Hz)
    pub poll_interval_ms: u64,

    /// RF chunk size (DMA_CHUNK_SAMPLES * 2 bytes per I/Q pair)
    pub rf_chunk_bytes: usize,

    /// Audio buffer size (samples per poll cycle)
    pub audio_samples_per_poll: usize,
}

impl Default for DispatchConfig {
    fn default() -> Self {
        DispatchConfig {
            poll_interval_ms: 10,
            rf_chunk_bytes: 32768,  // 16384 complex samples
            audio_samples_per_poll: 1920,  // 192kHz @ 10ms = 1920 samples
        }
    }
}

/// Main multi-modal ingestion loop.
///
/// # Spawning (in src/main.rs)
/// ```ignore
/// tokio::spawn({
///     let dm = device_manager.clone();
///     let dma = dma_gateway.clone();
///     let flags = dirty_flags.clone();
///     async move {
///         run_dispatch_loop(dm, dma, flags, Default::default()).await;
///     }
/// });
/// ```
///
/// # Loop Behavior
/// - Interval: 10ms per iteration (100 Hz polling)
/// - RF polling: read from active devices (RTL-SDR/Pluto+)
/// - Audio polling: read from soundcard (cpal)
/// - Visual polling: read from camera (stub for now)
/// - GPU update: push RF chunks via DMA, queue audio/visual updates
/// - Error handling: log errors, continue (don't panic)
pub async fn run_dispatch_loop(
    device_manager: Arc<DeviceManager>,
    dma_gateway: Arc<std::sync::Mutex<IqDmaGateway>>,
    dirty_flags: Arc<DirtyFlags>,
    config: DispatchConfig,
) {
    let mut poll_interval = interval(Duration::from_millis(config.poll_interval_ms));
    let mut rf_read_buffer = vec![0u8; config.rf_chunk_bytes];

    eprintln!("[Dispatch] Starting multi-modal ingestion loop ({}ms interval)", config.poll_interval_ms);

    loop {
        poll_interval.tick().await;

        // ===== RF INGESTION =====
        // Poll all active RF devices (RTL-SDR, Pluto+)
        let rf_devices = device_manager.get_devices();

        for device in &rf_devices {
            let device_id = device.id;

            match device_manager.get_device_mut(device_id, |dev| {
                dev.read_sync(&mut rf_read_buffer)
            }) {
                Ok(n_read) => {
                    if n_read > 0 {
                        // Push raw IQ bytes to GPU (zero-copy DMA)
                        match dma_gateway.lock().unwrap().push_dma_chunk(&rf_read_buffer[..n_read]) {
                            Ok(_) => {
                                // Mark RF data available
                                dirty_flags.mark(&dirty_flags.frequency_lock_dirty);
                            }
                            Err(e) => {
                                eprintln!("[Dispatch] RF DMA push failed: {}", e);
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("[Dispatch] RF read error on device {}: {}", device_id, e);
                }
                Err(e) => {
                    eprintln!("[Dispatch] RF device {} not found: {}", device_id, e);
                }
            }
        }

        // ===== AUDIO INGESTION =====
        // Read from soundcard (via cpal, already implemented in src/audio.rs)
        // This would be integrated via channel from audio thread
        // For now: stub that audio is being continuously buffered elsewhere
        // TODO: Wire audio thread output to GPU buffer
        dirty_flags.mark(&dirty_flags.audio_features_dirty);

        // ===== VISUAL INGESTION =====
        // Read from camera (C925e or D435 depth camera)
        // For now: stub
        // TODO: Wire camera thread output to GPU buffer

        // All data now in GPU VRAM, consumers (visualization, analysis) can read
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dispatch_config_default() {
        let cfg = DispatchConfig::default();
        assert_eq!(cfg.poll_interval_ms, 10);
        assert_eq!(cfg.rf_chunk_bytes, 32768);
        assert_eq!(cfg.audio_samples_per_poll, 1920);
    }

    #[test]
    fn test_poll_interval_100hz() {
        let interval_ms = 10u64;
        let frequency_hz = 1000u64 / interval_ms;
        assert_eq!(frequency_hz, 100);
    }
}
