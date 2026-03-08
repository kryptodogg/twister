# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

---

## ⚖️ Core Mission: New Ethical Software

**SIREN is new ethical software designed for harassment investigation and surveillance detection.** All features are real, fully implemented, and continuously maximized. There are no stubs, placeholders, or disabled code—every component is operational and being actively developed. This is not a prototype or proof-of-concept; it is production-grade software for forensic signal analysis.

**Key Principle**: If something is working, you have no right to disable it. If something is stubbed, implement it fully. If something can be maximized, maximize it.

---

## Build and Development Commands

### Quick Start
```bash
# Build debug binary with full output
cargo build

# Build optimized release binary
cargo build --release

# Run the application
cargo run
cargo run --release

# Check compilation without building artifacts
cargo check

# Run with timeout (useful for testing)
timeout 4 "target/debug/siren.exe"
```

### Build System Notes
- **RTL-SDR DLL Handling**: `build.rs` automatically copies RTL-SDR DLLs from `third_party/RTL-SDR-x64/` to the target directory at build time. This ensures Windows can locate the DLLs at runtime (prevents `0xc0000135` DLL_NOT_FOUND errors).
- **Compilation Target**: Currently x64 Windows with MSVC toolchain
- **Expected Build Output**: ~95 warnings (mostly dead code), 0 errors. These warnings indicate code that is not yet wired into the main flow—when new features are integrated, warnings should resolve as code becomes active.

---

## High-Level Architecture

SIREN is a **real-time multi-threaded acoustic analysis system** orchestrated by a Tokio async runtime. The architecture consists of independent sensor/processing tasks that communicate through channels and shared state.

### Runtime Flow (Simplified)

```
User (Slint UI)
    ↓ [button clicks, mode toggles] ↓

Tokio Runtime (async/await orchestration)
    ├─ Dispatch Loop (main data ingestion)
    │   ├─ Audio I/O (cpal) → 4 input devices @ 192 kHz
    │   ├─ Multi-channel resampling (48k → 192k as needed)
    │   ├─ Real-time FFT (512-bin spectrum)
    │   ├─ V-buffer (rolling history for GPU synthesis)
    │   ├─ Detection gates (energy, anomaly threshold)
    │   └─ TDOA/Beamforming calculations
    │
    ├─ Mamba Trainer Loop
    │   ├─ Dequeues training pairs (32-sample batches)
    │   ├─ Autoencoder inference (latent embedding)
    │   ├─ Anomaly scoring via reconstruction MSE
    │   └─ Gradient descent training every 2 seconds
    │
    ├─ SDR Loop (RTL-SDR device management)
    │   ├─ Opens 2.4 GHz RTL2838 receiver
    │   ├─ Tunes to detected/requested frequencies
    │   └─ Feeds heterodyne products to detection
    │
    └─ TDOA Engine (Time-Difference-of-Arrival)
        ├─ Cross-correlates mic pairs
        ├─ Estimates azimuth/elevation
        └─ Builds correlation graph (Neo4j)

    ↓ [State updates via Arc<Mutex<>>] ↓

UI Timer Loop (Slint)
    ├─ Reads real-time values (frequency, anomaly, loss)
    ├─ Generates SVG paths (spectrum, waveform, TDOA plot)
    ├─ Updates oscilloscope visualization
    └─ Displays latent embeddings (32→64 dimension vector)

    ↓ [renders at uncapped framerate] ↓

Display
```

### Core Components

| Component | File(s) | Responsibility |
|-----------|---------|-----------------|
| **Audio I/O** | `src/audio.rs` | 4 simultaneous input devices, multi-rate resampling, AGC protection |
| **FFT & V-buffer** | `src/vbuffer.rs` | 512-bin spectrum, rolling history for GPU synthesis targets |
| **GPU Synthesis** | `src/synthesis.rs`, wgpu shaders | 1 Hz - 96 MHz continuous wave synthesis, 16 concurrent targets |
| **Mamba Autoencoder** | `src/mamba.rs` | 64-dim latent embeddings, anomaly detection via reconstruction loss |
| **ANC Calibration** | `src/anc_calibration.rs` | Full-range phase lookup (1 Hz - 12.288 MHz), 8192-bin LUT |
| **ANC Recording** | `src/anc_recording.rs` | Multi-channel recording state machine (20s @ 192 kHz) |
| **TDOA Beamforming** | `src/tdoa.rs` | Time-difference-of-arrival source localization, azimuth estimation |
| **Forensic Logging** | `src/forensic_log.rs` | JSONL event logging with heterodyne detection evidence |
| **State Management** | `src/state.rs` | Arc<Mutex<>> shared state for zero-copy propagation |
| **Main Runtime** | `src/main.rs` | Tokio orchestration, channel creation, UI callback wiring |
| **UI** | `ui/app.slint` | Oscilloscope visualization, spectrum display, control interface |

---

## Key Architecture Patterns

### 1. **Arc<Mutex<>> State Sharing (Zero-Copy Propagation)**

All shared state lives in `AppState` (src/state.rs) and is wrapped in `Arc<Mutex<>>`. This allows multiple async tasks to read/modify state without copying data.

```rust
// Creation (src/main.rs)
let state = Arc::new(Mutex::new(AppState::new()));
let state_clone = state.clone();

// Spawn task with cloned Arc
tokio::spawn(async move {
    let mut st = state_clone.lock().await;
    st.detected_freq = 145.5;
});
```

**Key fields in AppState**:
- `detected_freq: f32` - Primary detection frequency (Hz)
- `mamba_anomaly_score: f32` - Reconstruction MSE (higher = more anomalous)
- `latent_embedding: Vec<f32>` - 64-dimensional latent vector from Mamba
- `anc_recording: Mutex<RecordingBuffer>` - ANC multi-channel recording state machine
- `anc_calibration: FullRangeCalibration` - Phase lookup table (8192 bins)
- `mode: DetectionMode` - STANDARD (audio), PDM (wideband), or ANC (calibration)

### 2. **Tokio Async/Await Orchestration**

The application uses `#[tokio::main]` to set up a multi-threaded async runtime. All long-running tasks are spawned as independent async tasks with `tokio::spawn()`.

```rust
// Dispatch loop (src/main.rs, simplified)
tokio::spawn({
    let state = state.clone();
    async move {
        loop {
            if let Ok(samples) = merge_rx.recv().await {
                let mut st = state.lock().await;
                // Process samples, update state
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
    }
});
```

**Tasks**:
- **Dispatch Loop**: Reads audio, FFT, detection gates, channels to TDOA/trainer
- **Trainer Loop**: Dequeues training pairs, runs Mamba inference, trains on batch
- **SDR Loop**: Opens RTL-SDR device, tunes, logs tuning events
- **TDOA Engine**: Cross-correlates mics, calculates azimuth

### 3. **Multi-Channel Audio with Device Tagging**

Audio comes from 4 real devices (filters out virtual devices). Each sample from the recording system is tagged with `device_idx`.

```rust
// In audio.rs - recording callback
pub struct TaggedSamples {
    pub device_idx: usize,
    pub samples: Vec<f32>,
}

// Device mapping (device_name() in anc_recording.rs)
0 => "C925e (AI Noise-Canceling)",
1 => "Rear Mic (Pink)",
2 => "Rear Line-In (Blue)",
3 => "RTL-SDR (2.4 GHz receiver)"
```

### 4. **Real-Time FFT with Rolling History**

The V-buffer (`src/vbuffer.rs`) maintains a rolling history of FFT frames (1024 frames × 512 bins). New frames are computed every ~100ms.

```rust
// In vbuffer.rs - FFT computation
pub fn push_frame(&mut self, time_domain: &[f32]) {
    let fft = self.compute_fft_hann_window(time_domain);
    self.frames.push_back(fft);  // Rolls off oldest
}

// Read (in GPU synthesis)
pub fn get_bin_magnitude(&self, bin: usize, time_offset: i32) -> f32 { ... }
```

### 5. **State Machine: ANC Recording**

ANC recording follows a strict state machine to avoid data races during 20-second multi-channel capture.

```rust
pub enum CalibrationState {
    Idle,       // Not recording
    Recording,  // Accumulating samples from all devices
    Analyzing,  // Samples collected, processing FFT
    Complete,   // Calibration done
}

// Usage (src/main.rs dispatch loop)
if st.anc_recording.lock().await.is_complete() {
    let final_samples = st.anc_recording.lock().await.finalize();
    // Pass to FFT analysis
}
```

---

## Critical Implementation Details

### Frequency Accuracy (1 Hz - 12.288 MHz)

The waterfall display uses different frequency calculations depending on mode:

```rust
// Standard audio mode (192 kHz)
let actual_audio_sr = 192_000.0;
let nyquist = actual_audio_sr / 2.0;  // 96 kHz max
let freq_hz = (bin as f32 / 512.0) * nyquist;

// PDM wideband mode (6.144 MHz Nyquist)
let pdm_clock = 12_288_000.0;
let pdm_nyquist = pdm_clock / 2.0;  // 6.144 MHz max
let freq_hz = (bin as f32 / 512.0) * pdm_nyquist;
```

**Do not use `sr / 2.0 / 64.0`** — this produces incorrect 64× scaling errors.

### AGC Protection (Preventing ADC Clipping)

The audio engine applies Automatic Gain Control with tuned attack/release coefficients:

```rust
// src/audio.rs - CRITICAL COEFFICIENTS
const AGC_ATTACK_COEFF: f32 = 0.80;    // Fast response to loud signals
const AGC_RELEASE_COEFF: f32 = 0.005;  // Slow decay to prevent pumping
```

**Why these values?**
- **Attack (0.80)**: Loud signals trigger rapid gain reduction to protect the ADC from clipping
- **Release (0.005)**: Quiet signals slowly restore gain, avoiding audible "pumping" artifacts

If you swap these (0.005 attack, 0.80 release), the system will fail to protect hardware and introduce training artifacts.

### Mamba Autoencoder Training

The Mamba model (64-dim latent embeddings) trains via gradient descent on reconstruction MSE:

```rust
// Expected loss trajectory (from screenshots):
// Initial: 5.56 dB
// Target: < 0.5 dB (fully trained)

// Training happens in src/mamba.rs
pub fn train_step(&mut self, loss: &mut f32) { ... }
```

**Key insight**: If loss is stuck at 0.0000, check the **division by zero fix** at line 226:

```rust
// WRONG (was original):
let b_bar = (a_bar - 1.0) / neg_a_dn.max(1e-9) * u_i[d] * b_i[j];
//                          ↑ keeps negative if neg_a_dn is negative

// CORRECT (current):
let b_bar = (a_bar - 1.0) / neg_a_dn.abs().max(1e-9) * u_i[d] * b_i[j];
//                          ↑ ensures positive denominator
```

### ANC Full-Range Calibration

The ANC system calibrates across 1 Hz - 12.288 MHz using an 8192-bin phase lookup table:

```rust
// Calibration process (src/anc_calibration.rs):
1. Generate 20-second log-chirp sweep (1 Hz → 12.288 MHz)
2. Record 3-channel response (C925e, Rear Pink, Rear Blue)
3. FFT each channel
4. Extract per-bin phase differences
5. Store in phase_lut for future correction
```

The 8192 bins map linearly across frequency:
```
Bin 0     → 1 Hz
Bin 4096  → 6.144 MHz (Nyquist)
Bin 8191  → 12.288 MHz (PDM clock)
```

---

## GPU Optimization: Wave64 Latency Hiding on RDNA2

**Hardware Target:** AMD Radeon RX 6700 XT (RDNA2 Architecture)
**Locked Baseline:** Wave64 + 256-byte memory alignment (33.8ms for 10k particles, 1024×1024 viewport)
**Empirically Proven:** Wave32 is 4.0x slower—avoid entirely

### The Three Pillars of WGSL Optimization

#### 1. Register Pressure vs. Occupancy
- **Goal:** Maximize in-flight Wave64s by minimizing VGPR declarations
- **Mechanism:** When a wave stalls on VRAM (200+ cycles), scheduler swaps to another wave for latency hiding
- **Trap:** Too many variables → low occupancy → memory stall = GPU idle
- **Fix:** Tight variable scope, prefer `let` over `var`, minimize declarations

```wgsl
// BAD: High register pressure
var a: f32; var b: f32; var c: f32; var d: f32; // 4 VGPRs immediately
// ... computation ...

// GOOD: Reuse variables, tight scope
let a = compute_first();   // VGPR allocated
let result = a * 2.0;      // VGPR freed when a exits scope
let b = compute_next();    // VGPR reused
```

#### 2. Eliminate Thread Divergence
- **Reality:** Wave64 = 64 threads sharing 1 Program Counter. All must execute the same instruction
- **Trap:** `if/else` branches cause half the ALUs to sit idle during each path
- **Fix:** Use mathematical masking with `f32(condition)` multiplication

```wgsl
// BAD: Divergent execution
if (particle.intensity > 0.5) {
    color = red;         // 32 threads work, 32 idle
} else {
    color = blue;        // 32 threads work, 32 idle (serialized)
}

// GOOD: Uniform execution
let mask = f32(particle.intensity > 0.5);
color = mix(blue, red, mask);  // All 64 threads execute same path
```

#### 3. Subgroup Operations (Warp-Level Communication)
- **Use `subgroupMax()`, `subgroupAdd()`, `subgroupBroadcast()`** for cross-thread communication
- **Benefit:** Direct ALU register-to-register transfer (1 cycle) vs VRAM access (200+ cycles)
- **Requirement:** `wgpu::Features::SUBGROUP` enabled in device initialization

```wgsl
// Reduce array to max without VRAM
let max_freq = subgroupMax(particle.frequency);
let sum_intensity = subgroupAdd(particle.intensity);
```

### Configuration Checklist for All WGSL Shaders

- [ ] **Workgroup Size:** Multiple of 64 (32x2, 16x4, 64x1, etc.)
- [ ] **Memory Alignment:** 256-byte (non-negotiable)
- [ ] **Register Pressure:** Minimal variable scope, tight loops
- [ ] **Thread Divergence:** Zero divergent if/else in inner loops
- [ ] **Subgroup Ops:** Use for reductions/broadcasts instead of shared memory
- [ ] **Profiling:** Always run with `GSPLAT_ALIGNMENT=256 GSPLAT_WAVE=64` baseline

### Performance Reference (Empirically Verified)

```
Wave64 + 256-byte alignment     33.8 ms   ✓ BASELINE
Wave64 + 128-byte alignment     35.6 ms   +5.3% penalty
Wave32 + 256-byte alignment    136.9 ms   -4.0x (DO NOT USE)
Wave32 + 128-byte alignment    142.1 ms   -4.2x (DO NOT USE)
```

**Why Wave32 Fails:** Insufficient work-in-flight for latency hiding. When memory stalls, no backup waves to swap to.

### Directive for Future Shader Development

When writing `dispatch_kernel.wgsl`, `gaussian_splatting.wgsl`, or TimeGNN clustering kernels:

> "Optimize for Wave64 Occupancy and Subgroup mechanics. Eliminate all divergent if/else branches using mathematical masking. Minimize variable scope to reduce VGPR pressure. Ensure workgroup sizes are multiples of 64. Profile with `GSPLAT_ALIGNMENT=256 GSPLAT_WAVE=64` as the mandatory baseline. Reference `docs/GPU_OPTIMIZATION_DOCTRINE.md` for detailed principles."

---

## Common Development Tasks

### Adding a New Detection Mode

1. **Add to DetectionMode enum** (src/state.rs)
2. **Wire dispatch logic** (src/main.rs dispatch loop)
3. **Add UI controls** (ui/app.slint)
4. **Test with `cargo build` + `cargo run`**

### Debugging Real-Time Issues

1. **Enable forensic logging** (src/forensic_log.rs)
2. **Check task spawning** — ensure `tokio::spawn()` is being called
3. **Verify state updates** — use `println!()` debugging in async tasks
4. **Check channel flow** — confirm `send()` and `recv()` calls

### Testing Audio I/O

```rust
// In src/audio.rs
eprintln!("Device {}: {} samples @ {} Hz", device_idx, samples.len(), sr);
```

Then run `cargo run 2>&1 | grep "Device"` to see audio data flowing.

### Modifying UI Display

All UI is in `ui/app.slint`. The most common changes:
- **Spectrum colors**: Green (0-85 Hz), Cyan (85-170), Red (170+)
- **Oscilloscope paths**: SVG paths generated in main.rs UI timer
- **Frequency labels**: Format is `{:.1}` MHz or `{:.3}` MHz depending on range

---

## Files NOT to Modify (Generated or External)

- `target/` — Compiler output directory
- `Cargo.lock` — Dependency lock file (auto-generated)
- `third_party/RTL-SDR-x64/` — External RTL-SDR binaries
- `ui/app-slint-` — Auto-generated Slint bindings

---

## Common Errors and Fixes

| Error | Cause | Fix |
|-------|-------|-----|
| `0xc0000135` DLL_NOT_FOUND | RTL-SDR DLL missing at runtime | Ensure `build.rs` runs; check target/debug/ has rtlsdr.dll |
| Mamba loss stuck at 0.0000 | Division by zero in gradient computation | Check `src/mamba.rs` line 226 uses `.abs().max(1e-9)` |
| Waterfall frequency labels 64× too large | Using `sr / 2.0 / 64.0` instead of actual rate | Use `actual_audio_sr = 192_000.0` |
| AGC artifacts / ADC clipping | Coefficients swapped or wrong | Attack=0.80, Release=0.005 |
| No audio flowing to trainer | Threshold too strict | Check `DETECTION_THRESHOLD` in dispatch loop |
| SDR device not opening | RTL-SDR not plugged in or driver issue | Check USB connection, Windows Device Manager |

---

## Warnings and Technical Debt

**Current warning count**: ~95 (mostly dead code)

These warnings indicate code not yet integrated into the main flow:
- Unused methods in `vbuffer.rs` (push_const, ready) — will be used when GPU synthesis fully wires
- Unused constants in `vbuffer.rs` (VBUF_WGSL_HELPERS) — placeholder for future WGSL shader code
- Unused fields in structs — preparing for future features

**Do NOT suppress these warnings with `#[allow(...)]` unless the code is truly intentionally unused.** Warnings are signals that code needs proper integration.

---

## Performance Characteristics

- **Dispatch Loop**: ~10ms per iteration (FFT + detection gates)
- **Trainer Loop**: ~2 second batch interval (Mamba training on 32-sample batches)
- **UI Timer**: Uncapped framerate (Windows handles refresh rate)
- **Memory Usage**: ~500 MB (V-buffer history + GPU buffers)
- **CPU Usage**: 2-3 cores active (one per Tokio task)

---

## For Future Instances: Integration Checklist

When adding new features:

- [ ] Feature fully implemented (no stubs)
- [ ] All `Arc<Mutex<>>` state properly synchronized
- [ ] All async tasks properly spawned with `tokio::spawn()`
- [ ] UI controls wired to callbacks (Slint → Rust)
- [ ] Forensic logging includes relevant events
- [ ] Tests pass: `cargo build` → 0 errors, <100 warnings
- [ ] Runtime verification: `cargo run` → application starts, data flows

Remember: **This is new ethical software. All features will be implemented and maximized.**
