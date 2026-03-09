# Candle Style Guide

## Purpose
Define standards for Candle lightweight inference in Project Twister, focusing on loading pre-trained models into GPU-first architecture with minimal latency for DSP feedback loops.

## Core Principles
1. **Sub-Millisecond Inference**: All model forward passes must complete within 1ms
2. **VRAM-Resident Models**: Pre-trained weights stay in GPU memory (no CPU↔GPU transfer during inference)
3. **Zero-Copy Input/Output**: Direct tensor sharing with DSP pipeline
4. **Quantization-First**: Use F16/I8 quantization where accuracy permits

## Model Loading Standards

### GPU-First Model Initialization
```rust
use candle_core::{Device, Tensor, DType};
use candle_nn::VarBuilder;

pub struct GpuModel {
    model: MyModel,
    device: Device,
    // Track VRAM usage for model weights
    vram_usage_mb: usize,
}

impl GpuModel {
    pub fn load_from_safetensors(
        path: &str,
        dtype: DType,
    ) -> Result<Self, candle_core::Error> {
        // Explicit GPU device selection (RX 6700 XT)
        let device = Device::new_cuda(0)?;

        // Load weights directly into VRAM (no CPU staging)
        let vb = VarBuilder::from_safetensors(
            vec![std::path::PathBuf::from(path)],
            dtype,
            &device,
        )?;

        let model = MyModel::new(vb)?;

        // Calculate VRAM usage
        let vram_usage_mb = model.param_count() * dtype.size() / (1024 * 1024);

        Ok(Self {
            model,
            device,
            vram_usage_mb,
        })
    }
}
```

### Model Quantization for DSP
```rust
use candle_core::quantization::QuantizedTensor;

// F16 quantization for spectrum encoder (50% VRAM reduction)
pub fn load_quantized_encoder(path: &str) -> Result<GpuModel, candle_core::Error> {
    GpuModel::load_from_safetensors(path, DType::F16)
}

// I8 quantization for classification head (75% VRAM reduction)
pub fn load_quantized_classifier(path: &str) -> Result<GpuModel, candle_core::Error> {
    GpuModel::load_from_safetensors(path, DType::Q8_0)
}
```

## DSP Integration Patterns

### Spectrum → Latent Encoding (≤1ms budget)
```rust
use std::time::Instant;

impl GpuModel {
    pub fn encode_spectrum(&self, spectrum: &[f32]) -> Result<Vec<f32>, candle_core::Error> {
        let start = Instant::now();

        // Zero-copy: wrap spectrum slice as GPU tensor (no CPU→GPU copy)
        let input = Tensor::from_slice(spectrum, (1, 256), &self.device)?;

        // Forward pass (F16 arithmetic)
        let latent = self.model.forward(&input)?;

        // Read back latent vector (32 f32 values)
        let latent_vec = latent.to_vec1::<f32>()?;

        let elapsed = start.elapsed();
        assert!(elapsed.as_millis() <= 1,
            "Inference took {:?} (budget: 1ms)", elapsed);

        Ok(latent_vec)
    }
}
```

### Batched Inference for Multi-Channel SDR
```rust
impl GpuModel {
    pub fn encode_batch(&self, spectra: &[Vec<f32>]) -> Result<Vec<Vec<f32>>, candle_core::Error> {
        let batch_size = spectra.len();

        // Stack spectra into single tensor (batch, 256)
        let flat: Vec<f32> = spectra.iter().flatten().cloned().collect();
        let input = Tensor::from_slice(&flat, (batch_size as u32, 256), &self.device)?;

        // Batch forward pass (more efficient than sequential)
        let latents = self.model.forward(&input)?;

        // Reshape back to Vec<Vec<f32>>
        let latents_vec = latents.to_vec2::<f32>()?;
        Ok(latents_vec)
    }
}
```

## Low-Latency Architecture

### Model Fusion for DSP Pipeline
```rust
pub struct DspInference {
    encoder: GpuModel,      // Spectrum → 32-dim latent
    classifier: GpuModel,   // Latent → modulation type
    vram_budget_mb: usize,
}

impl DspInference {
    pub fn new(encoder_path: &str, classifier_path: &str) -> Result<Self, candle_core::Error> {
        let encoder = GpuModel::load_from_safetensors(encoder_path, DType::F16)?;
        let classifier = GpuModel::load_from_safetensors(classifier_path, DType::F16)?;

        let total_vram = encoder.vram_usage_mb + classifier.vram_usage_mb;

        Ok(Self {
            encoder,
            classifier,
            vram_budget_mb: total_vram,
        })
    }

    // End-to-end latency: ≤2ms (encoder + classifier)
    pub fn classify_spectrum(&self, spectrum: &[f32]) -> Result<u32, candle_core::Error> {
        let latent = self.encoder.encode_spectrum(spectrum)?;
        let class_id = self.classifier.classify(&latent)?;
        Ok(class_id)
    }
}
```

### Async Inference Pipeline
```rust
use tokio::sync::mpsc;

pub struct AsyncInference {
    tx: mpsc::Sender<Vec<f32>>,
    rx: mpsc::Receiver<Result<Vec<f32>, candle_core::Error>>,
}

impl AsyncInference {
    pub fn spawn(model: Arc<GpuModel>) -> Self {
        let (in_tx, mut in_rx) = mpsc::channel::<Vec<f32>>(16);
        let (out_tx, out_rx) = mpsc::channel(16);

        tokio::spawn(async move {
            while let Some(spectrum) = in_rx.recv().await {
                let result = model.encode_spectrum(&spectrum);
                let _ = out_tx.send(result).await;
            }
        });

        Self {
            tx: in_tx,
            rx: out_rx,
        }
    }

    pub async fn infer(&mut self, spectrum: Vec<f32>) -> Result<Vec<f32>, candle_core::Error> {
        self.tx.send(spectrum).await.unwrap();
        self.rx.recv().await.unwrap()
    }
}
```

## Memory Safety Rules

### VRAM Budget Enforcement
```rust
pub struct VramBudget {
    total_mb: usize,
    used_mb: AtomicUsize,
}

impl VramBudget {
    pub fn new(total_mb: usize) -> Self {
        Self {
            total_mb,
            used_mb: AtomicUsize::new(0),
        }
    }

    pub fn allocate(&self, model_vram_mb: usize) -> Result<(), VramError> {
        let current = self.used_mb.load(Ordering::Relaxed);
        if current + model_vram_mb > self.total_mb {
            return Err(VramError::OutOfMemory);
        }
        self.used_mb.fetch_add(model_vram_mb, Ordering::Relaxed);
        Ok(())
    }
}

// Global VRAM budget: 4GB for ML models (out of 12GB total)
static VRAM_BUDGET: Lazy<VramBudget> = Lazy::new(|| VramBudget::new(4096));
```

### Model Unloading Policy
```rust
pub struct CachedModel {
    model: Option<GpuModel>,
    path: String,
    last_used: Instant,
    vram_mb: usize,
}

impl CachedModel {
    pub fn unload_if_idle(&mut self, idle_threshold: Duration) -> Result<(), candle_core::Error> {
        if self.last_used.elapsed() > idle_threshold {
            // Free VRAM by dropping model
            self.model = None;
            VRAM_BUDGET.deallocate(self.vram_mb);
        }
        Ok(())
    }
}
```

## Performance Benchmarks

### Target Metrics (RX 6700 XT + Ryzen 5700X)
| Model | Precision | Inference Time | VRAM Usage |
|-------|-----------|----------------|------------|
| Spectrum Encoder (256→32) | F16 | ≤0.5ms | 16 MB |
| Modulation Classifier (32→8) | F16 | ≤0.3ms | 8 MB |
| Mamba SSM Encoder | F16 | ≤1.0ms | 64 MB |
| Total Pipeline | F16 | ≤2ms | 128 MB |

### Latency Budget Breakdown
```
DSP Frame Budget: 5ms total
├─ RTL-SDR capture:     1ms
├─ FFT (2048 bins):     0.5ms
├─ ML Inference:        1ms    ← Candle budget
├─ Bispectrum:          1.5ms
└─ Waterfall render:    1ms
```

## Model Format Standards

### Safetensors for GPU Loading
```rust
// Preferred format: safetensors (zero-copy GPU loading)
use safetensors::SafeTensors;

pub fn load_model_weights(path: &str) -> Result<HashMap<String, Tensor>, candle_core::Error> {
    let file = std::fs::File::open(path)?;
    let buffer = unsafe { memmap2::MmapOptions::new().map(&file)? };
    let tensors = SafeTensors::deserialize(&buffer)?;

    // Direct GPU upload (no CPU intermediate)
    let mut weights = HashMap::new();
    for (name, tensor_view) in tensors.tensors() {
        let tensor = Tensor::from_buffer(
            tensor_view.data(),
            tensor_view.shape(),
            tensor_view.dtype(),
            &Device::Cuda(0),
        )?;
        weights.insert(name, tensor);
    }

    Ok(weights)
}
```

## Integration with Qdrant/Neo4j

### Latent Storage Pipeline
```rust
pub async fn store_latent_in_qdrant(
    qdrant_client: &QdrantClient,
    latent: &[f32],
    metadata: &DetectionMetadata,
) -> Result<(), qdrant_client::error::Error> {
    // Normalize latent vector (L2 norm)
    let norm = latent.iter().map(|v| v * v).sum::<f32>().sqrt();
    let normalized: Vec<f32> = latent.iter().map(|v| v / norm).collect();

    // Store in Qdrant (32-dim vector)
    qdrant_client
        .upsert_points(
            "rf_signals",
            vec![PointStruct::new(
                metadata.id,
                normalized,
                metadata.to_payload(),
            )],
        )
        .await?;

    Ok(())
}
```

## References
- [Candle Documentation](https://huggingface.co/docs/candle/index)
- [Safetensors Format Specification](https://github.com/huggingface/safetensors)
- [Candle Quantization Guide](https://github.com/huggingface/candle/blob/main/candle-core/src/quantization.rs)
