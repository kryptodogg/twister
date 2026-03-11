# Twister: Active Noise Cancellation with BBS & Mamba Autoencoder

## Project Overview

Real-time ANC system combining:
- **Blind Source Separation (BSS)** with RLS adaptive filtering
- **Mamba State-Space Model** autoencoder for signal separation
- **RTL-SDR + Sound Card** dual-input capture
- **<35ms end-to-end latency** target

## Architecture

```
src/
├── main.rs                 # Entry point, pipeline orchestration
├── lib.rs                  # Library exports
├── bin/
│   ├── calibrate.rs        # Hardware calibration utility
│   └── latency_test.rs     # Latency benchmark tool
├── hardware/
│   ├── mod.rs
│   ├── audio.rs            # CPAL sound card capture/playback
│   ├── rtlsdr.rs           # RTL-SDR signal capture
│   ├── traits.rs           # Hardware abstraction traits
│   └── calibration.rs      # Device calibration & verification
├── bss/
│   ├── mod.rs
│   ├── rls.rs              # Recursive Least Squares estimator
│   ├── lms.rs              # LMS adaptive filter (fallback)
│   ├── beamformer.rs       # Spatial filtering
│   └── traits.rs           # BSS algorithm traits
├── mamba/
│   ├── mod.rs
│   ├── model.rs            # Mamba SSM architecture
│   ├── encoder.rs          # Signal encoding
│   ├── decoder.rs          # Signal reconstruction
│   ├── training.rs         # Training pipeline (PyO3 bridge)
│   └── inference.rs        # Real-time inference
├── pipeline/
│   ├── mod.rs
│   ├── processor.rs        # Main signal processing pipeline
│   ├── buffer.rs           # Ring buffer management
│   ├── latency.rs          # Latency monitoring & profiling
│   └── sync.rs             # Cross-device synchronization
├── dsp/
│   ├── mod.rs
│   ├── fft.rs              # FFT utilities
│   ├── filters.rs          # FIR/IIR filter implementations
│   ├── window.rs           # Window functions
│   └── resample.rs         # Sample rate conversion
├── ml/
│   ├── mod.rs
│   ├── tensors.rs          # Tensor conversions
│   ├── pytorch.rs          # PyO3 bridge to TensorFlow/PyTorch
│   └── burn_backend.rs     # Burn ML backend
└── utils/
    ├── mod.rs
    ├── error.rs            # Error types
    ├── config.rs           # Configuration management
    └── logging.rs          # Tracing setup
```

## Data Flow

```
┌─────────────┐    ┌─────────────┐
│  RTL-SDR    │    │ Sound Card  │
│  (RF Input) │    │ (Mic Input) │
└──────┬──────┘    └──────┬──────┘
       │                  │
       ▼                  ▼
┌─────────────────────────────────┐
│     Hardware Abstraction Layer   │
│  - Sample rate synchronization   │
│  - Buffer management             │
│  - IQ → Audio conversion         │
└────────────────┬────────────────┘
                 │
                 ▼
┌─────────────────────────────────┐
│    BSS Module (RLS Estimator)   │
│  - Adaptive noise cancellation   │
│  - Source separation             │
│  - Beamforming (multi-antenna)   │
└────────────────┬────────────────┘
                 │
                 ▼
┌─────────────────────────────────┐
│   Mamba Autoencoder (SSM)       │
│  - Temporal pattern learning     │
│  - Signal reconstruction         │
│  - Feature extraction            │
└────────────────┬────────────────┘
                 │
                 ▼
┌─────────────────────────────────┐
│    Noise Suppression Block      │
│  - Spectral subtraction          │
│  - Wiener filtering              │
└────────────────┬────────────────┘
                 │
                 ▼
┌─────────────────────────────────┐
│     Output (Speakers/Stream)    │
└─────────────────────────────────┘
```

## Latency Budget (<35ms target)

| Stage                    | Budget  | Notes                           |
|-------------------------|---------|--------------------------------|
| Hardware capture         | 5ms     | Buffer size ~256 samples @48kHz |
| BSS/RLS processing       | 10ms    | Adaptive filter convergence     |
| Mamba inference          | 12ms    | State-space model forward pass  |
| Noise suppression        | 3ms     | Spectral processing             |
| Output buffering         | 3ms     | Playback buffer                 |
| **Margin**               | **2ms** | System overhead                 |
| **Total**                | **35ms**|                                 |

## Hardware Requirements

### RTL-SDR Dongle
- **Chipset**: R820T2 or E4000
- **Driver**: RTL-SDR.dll (Windows) or librtlsdr (Linux)
- **Sample Rate**: 2.4 MSPS max (use 1-2 MSPS for stability)
- **Frequency Range**: 24-1766 MHz
- **Antenna**: SMA connector, consider magnetic mount for mobility

### Sound Card
- **Interface**: USB or built-in
- **Sample Rate**: 48kHz or 96kHz
- **Channels**: Stereo minimum, 4+ for beamforming
- **Latency**: ASIO driver preferred on Windows

### Optional: PlutoSDR
- **Full-duplex**: Simultaneous TX/RX
- **Frequency**: 325 MHz - 6 GHz
- **Bandwidth**: 20 MHz

## ML Backend Options

### Option A: Burn (Pure Rust) - RECOMMENDED for inference
```toml
burn = { version = "0.16", features = ["ndarray", "autodiff"] }
```
- ✅ Zero Python dependency
- ✅ Native Rust performance
- ✅ Mamba/SSM support via custom modules
- ❌ Training slower than PyTorch

### Option B: PyO3 + TensorFlow/PyTorch
```toml
pyo3 = { version = "0.22", features = ["auto-initialize"] }
```
- ✅ Full TensorFlow/PyTorch ecosystem
- ✅ Pre-trained Mamba models available
- ✅ Faster training
- ❌ Python runtime required
- ❌ GIL overhead for real-time

**Recommendation**: Train in Python (TensorFlow), export to ONNX, run inference in Rust via Burn or ONNX Runtime.

## Implementation Phases

### Phase 1: Hardware Setup (Tasks 1-3, 55 min)
1. Verify RTL-SDR detection and streaming
2. Configure sound card with low-latency drivers
3. Test synchronized capture from both devices

### Phase 2: BSS Module (Tasks 4, 20 min)
1. Implement RLS adaptive filter
2. Configure reference noise input
3. Test convergence on synthetic data

### Phase 3: Mamba Integration (Task 5, 15 min)
1. Set up Burn backend or PyO3 bridge
2. Load pre-trained Mamba model
3. Integrate into processing pipeline

### Phase 4: Integration & Testing (Tasks 6-8, 35 min)
1. Add noise suppression block
2. Run end-to-end latency test
3. Document results and tuning parameters

## Key Design Decisions

1. **Sample Rate**: 48kHz audio, 1-2 MSPS RTL-SDR (downconvert for audio ANC)
2. **Buffer Size**: 256-512 samples for <10ms capture latency
3. **RLS Order**: 32-64 taps for balance of convergence vs. computation
4. **Mamba Dimensions**: d_model=64, n_layers=4 for real-time inference
5. **Threading**: Separate capture, processing, and playback threads

## Next Steps

1. Run `cargo build` to verify dependencies
2. Connect RTL-SDR and run hardware detection
3. Run calibration utility to measure device latencies
4. Begin BSS module implementation
