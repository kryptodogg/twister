# Rust Style Guide

## Purpose
Define Rust coding standards for Project Twister, prioritizing memory safety, zero-copy operations, and real-time DSP performance.

## Core Principles
1. **Memory Safety First**: No `unsafe` without explicit justification and audit trail
2. **Zero-Copy Where Possible**: Leverage `Arc`, `Mutex`, and memory-mapped I/O
3. **Deterministic Latency**: No unbounded allocations in DSP hot paths
4. **Explicit Error Handling**: `anyhow::Result` for application, `thiserror` for libraries

## Code Organization

### Module Structure
```rust
// src/lib.rs or src/main.rs
pub mod audio;      // Audio I/O (CPAL, WASAPI)
pub mod dsp;        // DSP core (FFT, filters, PDM)
pub mod gpu;        // GPU compute (WGPU, CubeCL)
pub mod ml;         // ML inference (Burn, Candle)
pub mod db;         // Database clients (Neo4j, Qdrant)
pub mod ui;         // Slint UI bindings
```

### Import Conventions
```rust
// Standard library (alphabetical)
use std::sync::{Arc, Mutex, atomic::{AtomicBool, AtomicU32, Ordering}};
use std::time::{Duration, Instant};

// External crates (grouped by purpose)
use anyhow::{Context, Result};
use candle_core::{Device, Tensor};
use slint::{SharedString, VecModel};
use wgpu::{Device as WgpuDevice, Queue};

// Internal modules (relative paths)
use crate::dsp::fft::FftPlanner;
use crate::gpu::waterfall::WaterfallEngine;
```

## Memory Safety Rules

### Allocation Guidelines
```rust
// ✅ GOOD: Pre-allocated buffers with capacity
pub struct DspBuffer {
    samples: Vec<f32>,
    capacity: usize,
}

impl DspBuffer {
    pub fn new(capacity: usize) -> Self {
        let mut samples = Vec::with_capacity(capacity);
        samples.resize(capacity, 0.0); // Pre-allocate
        Self { samples, capacity }
    }
}

// ❌ BAD: Unbounded growth in DSP loop
fn process_frame(samples: &mut Vec<f32>) {
    samples.push(new_sample); // May reallocate → latency spike
}
```

### Arc/Mutex Patterns
```rust
// ✅ GOOD: Lock-free atomics for shared state
pub struct SharedState {
    pub running: AtomicBool,
    pub frame_count: AtomicU32,
    pub detected_freq: AF32, // AtomicF32
}

// ✅ GOOD: Fine-grained locking for buffers
pub struct GpuShared {
    pub waterfall_rgba: Mutex<Vec<u32>>, // Only locked during readback
    pub spectrum: Mutex<Vec<f32>>,
}

// ❌ BAD: Coarse-grained locking (blocks entire pipeline)
pub struct BadShared {
    pub everything: Mutex<Everything>, // Locks all state
}
```

## Error Handling

### Application Layer (anyhow)
```rust
use anyhow::{Context, Result};

pub fn init_audio_device() -> Result<AudioDevice> {
    let host = cpal::default_host();
    host.default_output_device()
        .context("No audio output device found")?
        .build_stream(...)
        .context("Failed to build audio stream")
}
```

### Library Layer (thiserror)
```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DspError {
    #[error("FFT size mismatch: expected {expected}, got {actual}")]
    FftSizeMismatch { expected: usize, actual: usize },

    #[error("GPU buffer overflow: {0} bytes")]
    GpuBufferOverflow(usize),

    #[error("WGPU error: {0}")]
    Wgpu(#[from] wgpu::Error),
}
```

## Performance Patterns

### Zero-Copy DSP Buffers
```rust
// ✅ GOOD: Ring buffer with memory-mapped I/O
use ringbuf::{HeapRb, Rb};

pub struct AudioStream {
    ring: HeapRb<f32>,
}

impl AudioStream {
    pub fn write_samples(&mut self, samples: &[f32]) -> usize {
        self.ring.push_slice(samples) // Zero-copy
    }

    pub fn read_samples(&mut self, buf: &mut [f32]) -> usize {
        self.ring.pop_slice(buf) // Zero-copy
    }
}
```

### Lock-Free Communication
```rust
use crossbeam_channel::{bounded, Sender, Receiver};

// ✅ GOOD: Bounded channel for frame communication
pub fn create_frame_channel() -> (Sender<Frame>, Receiver<Frame>) {
    bounded(16) // Backpressure: drop frames if consumer is slow
}

// ❌ BAD: Unbounded channel (memory leak risk)
use crossbeam_channel::unbounded; // Never use in DSP pipeline
```

## Naming Conventions

### Types and Traits
```rust
// Structs: PascalCase
pub struct WaterfallEngine;
pub struct BispectrumAnalyzer;

// Traits: descriptive nouns
pub trait SignalProcessor {
    fn process(&mut self, input: &[f32]) -> Vec<f32>;
}

// Enums: PascalCase with descriptive variants
pub enum DenialMode {
    Off,
    AntiPhase,
    Noise,
    Tone,
    Sweep,
    AncAntiPhase,
}
```

### Functions and Methods
```rust
// Snake_case, verb-first for actions
pub fn process_frame(samples: &[f32]) -> Vec<f32>;
pub fn calculate_coherence(spectrum: &[f32]) -> f32;

// Getters/setters (no get_/set_ prefix for simple accessors)
impl State {
    pub fn detected_freq(&self) -> f32 { self.freq.load() }
    pub fn set_detected_freq(&self, freq: f32) { self.freq.store(freq); }
}
```

## Documentation Standards

### Module-Level Docs
```rust
//! # DSP Module
//!
//! Real-time digital signal processing for RF analysis.
//!
//! ## Components
//! - FFT (2048-bin, Hann window)
//! - PDM→PCM conversion (64× oversampling)
//! - Bispectrum coherence analysis
//!
//! ## Performance Budget
//! - FFT: ≤0.5ms
//! - PDM decode: ≤1ms
//! - Bispectrum: ≤2ms
```

### Function Documentation
```rust
/// Encode spectrum into 32-dim latent vector for Qdrant storage.
///
/// # Arguments
/// * `spectrum` - Log-mapped FFT magnitudes (256 bins)
///
/// # Returns
/// * `Ok(Vec<f32>)` - Normalized latent vector (L2 norm = 1.0)
/// * `Err(ModelError)` - If inference fails or exceeds 1ms budget
///
/// # Performance
/// - Target latency: ≤1ms (RX 6700 XT)
/// - VRAM usage: 16 MB
pub fn encode_spectrum(&self, spectrum: &[f32]) -> Result<Vec<f32>>;
```

## Testing Standards

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fft_roundtrip() {
        let signal = vec![1.0, 0.0, -1.0, 0.0];
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(4);

        let mut spectrum = fft.process(&signal);
        let reconstructed = fft.process_inverse(&spectrum);

        assert_relative_eq!(signal, reconstructed, epsilon = 1e-6);
    }
}
```

### Integration Tests
```rust
#[cfg(test)]
mod integration {
    #[test]
    fn test_full_pipeline_latency() {
        let start = Instant::now();

        // Capture → FFT → Bispectrum → ML → Waterfall
        run_full_pipeline();

        let elapsed = start.elapsed();
        assert!(elapsed < Duration::from_millis(5),
            "Pipeline took {:?} (budget: 5ms)", elapsed);
    }
}
```

## References
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Rust By Example](https://doc.rust-lang.org/rust-by-example/)
- [Rust Performance Book](https://nnethercote.github.io/perf-book/)
