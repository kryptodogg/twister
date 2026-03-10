// src/dispatch/mod.rs — IQ Sample Dispatch Pipeline
//
// Tokio-based dispatch loop for streaming IQ samples from RTL-SDR devices to GPU.
// Zero-copy architecture: raw [u8; 2] IQ bytes → DMA → GPU FFT → spectral history.

pub mod iq_dispatch;

pub use iq_dispatch::IqDispatchLoop;
