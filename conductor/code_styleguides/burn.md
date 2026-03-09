# Burn 0.21-pre2 Style Guide

## Purpose
Define standards for Burn ML framework usage in Project Twister's SDR/ML pipeline, with focus on WGPU backend optimizations for real-time RF signal processing (10 KHz - 300 MHz).

## Core Principles
1. **GPU-First Architecture**: All tensor operations must target WGPU backend
2. **Streaming Data**: Never load full RF captures into VRAM
3. **Zero-Copy Where Possible**: Leverage Burn's `TensorData` for direct memory mapping
4. **Deterministic Latency**: ML inference must complete within DSP frame budget (≤5ms)

## WGPU Backend Configuration

### Device Initialization
```rust
use burn::backend::wgpu::{WgpuDevice, WgpuRuntime};
use burn::backend::Burn;

// Explicit device selection for AMD SAM (Smart Access Memory)
let device = WgpuDevice::DiscreteGpu(0); // RX 6700 XT
let config = burn::backend::wgpu::WgpuConfig {
    device,
    memory_config: burn::backend::wgpu::MemoryConfig {
        // Unified memory access via AMD SAM
        unified_memory: true,
        // Reserve VRAM for waterfall + bispectrum
        reserved_vram_mb: 2048,
    },
};
```

### Tensor Definitions for RF Data

#### Spectrum Tensor (256-bin FFT)
```rust
use burn::tensor::{Tensor, Data, Shape};
use burn::backend::TensorPrimitive;

// Log-mapped magnitude spectrum: [batch, bins]
pub type SpectrumTensor = Tensor<burn::tensor::Float, 2>;

// Construction from RTL-SDR FFT output
pub fn spectrum_from_fft(fft_bins: &[f32]) -> SpectrumTensor {
    Tensor::from_data(
        Data::new(fft_bins.to_vec(), Shape { dims: [1, 256] }),
        &Default::default(),
    )
}
```

#### Bispectrum Tensor (O(N²) coherence matrix)
```rust
// Bispectrum coherence: [batch, f1, f2]
pub type BispectrumTensor = Tensor<burn::tensor::Float, 3>;

// Lazy evaluation - only compute significant bins
pub fn bispectrum_sparse(spectrum: &SpectrumTensor, threshold: f32) -> BispectrumTensor {
    // Implement sparse coherence calculation
    // Skip bins below threshold to reduce O(N²) → O(kN)
    todo!()
}
```

## Streaming Data Loaders for RTL-SDR

### RF Sample Stream
```rust
use burn::data::dataloader::DataLoader;
use burn::data::dataset::Dataset;

pub struct RfSampleDataset {
    sample_rate: f32,
    center_freq: f32,
    // Ring buffer for streaming - never owns full capture
    buffer: Arc<Mutex<RingBuffer<f32>>>,
}

impl Dataset<RfSample> for RfSampleDataset {
    fn len(&self) -> usize {
        // Streaming dataset - always "infinite"
        usize::MAX
    }

    fn get(&self, index: usize) -> Option<RfSample> {
        // Pull from ring buffer (non-blocking)
        self.buffer.lock().get(index)
    }
}

pub struct RfSample {
    pub iq_samples: Tensor<burn::tensor::Float, 1>,
    pub timestamp: u64,
    pub center_freq: f32,
}
```

### Memory Safety Rules
1. **No `.collect()` on RF streams**: Always use iterators with explicit bounds
2. **VRAM Budget**: Each tensor allocation must check `available_vram()` first
3. **Drop Policy**: Implement `Drop` for all RF tensors to free VRAM immediately

```rust
impl Drop for RfSample {
    fn drop(&mut self) {
        // Explicit VRAM release
        self.iq_samples.detach();
    }
}
```

## Modular Tensor Definitions

### RF-Specific Tensor Types
```rust
// Complex I/Q samples (interleaved real/imag)
pub type IQTensor = Tensor<burn::tensor::Float, 1>;

// Power spectral density (dBFS)
pub type PS DTensor = Tensor<burn::tensor::Float, 1>;

// Phase coherence matrix (bispectrum)
pub type PhaseTensor = Tensor<burn::tensor::Float, 2>;

// Latent embedding (32-dim for Qdrant storage)
pub type LatentTensor = Tensor<burn::tensor::Float, 1>;

impl LatentTensor {
    pub fn normalize_l2(&self) -> Self {
        let norm = self.powf_scalar(2.0).sum().sqrt();
        self.div(norm)
    }

    pub fn to_qdrant_vec(&self) -> Vec<f32> {
        self.to_data().convert().to_vec()
    }
}
```

## Training Loop Standards

### Real-Time Fine-Tuning
```rust
use burn::train::TrainOutput;
use burn::optim::Optimizer;

pub struct RealTimeTrainer<B: Backend> {
    model: MyModel<B>,
    optimizer: AdamConfig,
    // Frame budget: 5ms max training time
    frame_budget: Duration,
}

impl<B: Backend> RealTimeTrainer<B> {
    pub fn train_step(&mut self, batch: RfSample) -> TrainOutput {
        let start = std::time::Instant::now();

        // Forward pass
        let output = self.model.forward(batch.iq_samples);

        // Compute loss (e.g., reconstruction error)
        let loss = output.mse_loss(&batch.iq_samples);

        // Backward pass with gradient clipping
        let mut gradients = loss.backward();
        gradients.clip_(1.0); // Prevent explosion in RF noise

        // Update weights
        self.optimizer.step(&mut self.model, &mut gradients);

        // Enforce frame budget
        assert!(start.elapsed() < self.frame_budget,
            "Training step exceeded {}ms budget", self.frame_budget.as_millis());

        TrainOutput::new(self.model.clone(), loss)
    }
}
```

## Performance Benchmarks

### Target Metrics (RX 6700 XT + Ryzen 5700X)
| Operation | Target Latency | VRAM Usage |
|-----------|---------------|------------|
| FFT (2048 bins) | ≤0.5ms | 8 MB |
| Bispectrum (sparse) | ≤2ms | 64 MB |
| Mamba Encoder (32-dim) | ≤1ms | 16 MB |
| Training Step (batch=1) | ≤5ms | 128 MB |
| Total Pipeline | ≤10ms | 256 MB |

### Memory Safety Checklist
- [ ] All tensors have explicit `Shape` definitions
- [ ] Streaming loaders use `RingBuffer` (no Vec growth)
- [ ] VRAM budget checked before each allocation
- [ ] `Drop` implemented for all RF tensor types
- [ ] Gradient clipping enabled for training loops
- [ ] Frame budget assertions in training steps

## References
- [Burn 0.21-pre2 Documentation](https://burn.dev/docs/burn/)
- [WGPU Memory Management](https://docs.rs/wgpu/latest/wgpu/)
- [AMD Smart Access Memory Technical Brief](https://www.amd.com/en/technologies/smart-access-memory)
