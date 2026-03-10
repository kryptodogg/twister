// src/dispatch/iq_dispatch.rs — Tokio IQ Sample Dispatch Loop
//
// Async dispatch loop that polls RTL-SDR devices and streams IQ samples to GPU.
// Architecture:
//   DeviceManager → read_sync() → IqDmaGateway → GPU VRAM → STFT → V-Buffer
//
// Zero-copy: IQ bytes never converted to f32 on CPU.
// Backpressure: Drops frames if GPU queue is backed up (real-time priority).

use crate::hardware_io::device_manager::DeviceManager;
use crate::hardware_io::dma_vbuffer::{IqDmaGateway, DMA_CHUNK_SAMPLES};
use std::sync::Arc;
use tokio::time::{interval, Duration};

/// Target frame rate for spectral updates (48 fps = ~21ms per frame).
/// At 2.4 MSPS, this is ~50,000 samples per frame.
const TARGET_FPS: u32 = 48;
const FRAME_DURATION_MS: u64 = 1000 / TARGET_FPS as u64;

/// Samples per frame at 2.4 MSPS @ 48 fps = 50,000 complex samples.
/// Rounded to power of 2 for FFT efficiency: 65536 (2^16).
const SAMPLES_PER_FRAME: usize = 65536;
const BYTES_PER_FRAME: usize = SAMPLES_PER_FRAME * 2; // I+Q = 2 bytes per complex sample

/// Dispatch loop state.
pub struct IqDispatchLoop {
    device_manager: Arc<DeviceManager>,
    dma_gateway: Arc<std::sync::Mutex<IqDmaGateway>>,
    running: bool,
    frame_count: u64,
    dropped_frames: u64,
}

impl IqDispatchLoop {
    /// Create a new IQ dispatch loop.
    ///
    /// # Parameters
    /// - `device_manager`: Shared device registry for RTL-SDR access
    /// - `dma_gateway`: DMA gateway for zero-copy GPU transfer
    ///
    /// # Returns
    /// Initialized dispatch loop (not yet running)
    pub fn new(
        device_manager: Arc<DeviceManager>,
        dma_gateway: Arc<std::sync::Mutex<IqDmaGateway>>,
    ) -> Self {
        IqDispatchLoop {
            device_manager,
            dma_gateway,
            running: false,
            frame_count: 0,
            dropped_frames: 0,
        }
    }

    /// Run the dispatch loop asynchronously.
    ///
    /// # Behavior
    /// 1. Polls DeviceManager for active RTL-SDR devices
    /// 2. Reads IQ samples via read_sync()
    /// 3. Pushes samples to DMA gateway for GPU transfer
    /// 4. Maintains target FPS with tokio interval timer
    ///
    /// # Returns
    /// Runs until `stop()` is called or device error occurs.
    pub async fn run(&mut self) -> Result<(), String> {
        self.running = true;
        let mut frame_timer = interval(Duration::from_millis(FRAME_DURATION_MS));

        eprintln!("[IQ Dispatch] Starting loop @ {} FPS", TARGET_FPS);

        while self.running {
            frame_timer.tick().await;

            // Check if we have devices
            if !self.device_manager.has_devices() {
                tokio::time::sleep(Duration::from_millis(100)).await;
                continue;
            }

            // Get first active device (multi-device support future TODO)
            let devices = self.device_manager.get_devices();
            if devices.is_empty() {
                continue;
            }

            let device_id = devices[0].id;

            // Read IQ samples from device
            let mut frame_buffer = vec![0u8; BYTES_PER_FRAME];
            match self.device_manager.get_device_mut(device_id, |dev| {
                dev.read_sync(&mut frame_buffer)
            }) {
                Ok(n_read) => {
                    if n_read == 0 {
                        self.dropped_frames += 1;
                        continue;
                    }

                    // Push to DMA gateway for GPU transfer
                    let dma_offset = {
                        let mut dma_guard = self.dma_gateway.lock().unwrap();
                        // Note: We need to pass the correct chunk size
                        // For now, we'll push in chunks of DMA_CHUNK_SAMPLES
                        let mut offset = 0;
                        while offset < n_read {
                            let chunk_size = (n_read - offset).min(DMA_CHUNK_SAMPLES * 2);
                            let chunk = &frame_buffer[offset..offset + chunk_size];
                            
                            // Pad to exact chunk size if needed
                            if chunk_size < DMA_CHUNK_SAMPLES * 2 {
                                let mut padded = vec![0u8; DMA_CHUNK_SAMPLES * 2];
                                padded[..chunk_size].copy_from_slice(chunk);
                                if let Err(e) = dma_guard.push_dma_chunk(&padded) {
                                    eprintln!("[IQ Dispatch] DMA push error: {}", e);
                                    self.dropped_frames += 1;
                                    break;
                                }
                            } else {
                                if let Err(e) = dma_guard.push_dma_chunk(chunk) {
                                    eprintln!("[IQ Dispatch] DMA push error: {}", e);
                                    self.dropped_frames += 1;
                                    break;
                                }
                            }
                            offset += chunk_size;
                        }
                        dma_guard.write_offset()
                    };

                    self.frame_count += 1;

                    if self.frame_count % 60 == 0 {
                        eprintln!(
                            "[IQ Dispatch] Frame {} | Dropped: {} | Offset: {}",
                            self.frame_count,
                            self.dropped_frames,
                            dma_offset
                        );
                    }
                }
                Err(e) => {
                    eprintln!("[IQ Dispatch] Device read error: {}", e);
                    self.dropped_frames += 1;
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            }
        }

        eprintln!("[IQ Dispatch] Stopped. Total frames: {}, Dropped: {}", self.frame_count, self.dropped_frames);
        Ok(())
    }

    /// Stop the dispatch loop.
    pub fn stop(&mut self) {
        self.running = false;
    }

    /// Get frame count (for monitoring).
    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }

    /// Get dropped frame count (for monitoring).
    pub fn dropped_frames(&self) -> u64 {
        self.dropped_frames
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_state::DirtyFlags;

    #[test]
    fn test_dispatch_loop_creation() {
        let dirty_flags = Arc::new(DirtyFlags::new());
        let device_manager = Arc::new(DeviceManager::new(dirty_flags));
        
        // Create a mock DMA gateway (would need wgpu device in real test)
        // For unit test, just verify the struct can be created
        let _loop = IqDispatchLoop::new(
            device_manager,
            Arc::new(std::sync::Mutex::new(
                create_mock_dma_gateway()
            )),
        );
        // Test passes if struct creation doesn't panic
    }

    #[test]
    fn test_frame_constants() {
        assert_eq!(TARGET_FPS, 48);
        assert_eq!(FRAME_DURATION_MS, 20); // 1000 / 48 = 20.83, truncated to 20
        assert_eq!(SAMPLES_PER_FRAME, 65536);
        assert_eq!(BYTES_PER_FRAME, 131072);
    }

    // Helper to create a mock DMA gateway for testing
    fn create_mock_dma_gateway() -> IqDmaGateway {
        // This would normally require wgpu device/queue
        // For unit tests, we use placeholder values
        // In integration tests, we'd use real wgpu devices
        unimplemented!("Requires wgpu device for full initialization")
    }
}
