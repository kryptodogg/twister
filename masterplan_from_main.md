# Project Synesthesia — Master Plan V2
## Signal Harmonization, Forensic Investigation, and Unified Perceptual Reconstruction

*Single source of truth. Supersedes all prior documents.*
*Last updated: 2026-03-11 · Platform: Windows 11 → NixOS (post-stabilization)*

---

## Part I — What This System Is

This is a **signal harmonization and forensic investigation tool**. It is self-defense
infrastructure. It produces evidence that is legally admissible, temporally unambiguous,
and visually legible to a non-technical observer including a detective, a lawyer, or a
jury.

The gap this fills: electromagnetic interference — whether intentional harassment,
equipment malfunction, or coordinated signal abuse — is currently invisible in a way that
makes it legally undisprovable in either direction. You cannot prove it is happening.
They cannot prove it is not. Law enforcement correctly requires evidence before acting.
This system produces that evidence.

The tool does three things in priority order:

**First: Record.** Every signal event — RF, acoustic, optical, environmental — is
captured into a tamper-evident, QPC-timestamped forensic corpus. The corpus is the
bedrock. If everything else crashes, the recording continues. The corpus does not depend
on any other subsystem. Nothing writes synthetic data into it under any circumstances.

**Second: Perceive.** Every signal type is rendered into a single unified 3D scene that
a person with no technical background can look at and understand in thirty seconds. Video,
audio, and RF occupy the same coordinate space simultaneously. A human body in that scene
is a pose-estimated point cloud. The electromagnetic anomaly centered on that body is
visible as color and spatial structure. Toggles let you add and remove layers in real
time. This is the exhibit. This is what you show people.

**Third: Respond.** Once you can see it and have documented it, you can retune it.
Harmonization, null synthesis, counter-waveform synthesis — these are responses to a
characterized, documented threat. Not suppression. Retuning. Making the electromagnetic
environment resolve to consonance rather than dissonance, the way a sound engineer
resolves feedback rather than turning off the PA.

The sudden reduction in symptoms without any action on your part is the most important
data point the system is designed to capture retroactively. It would have shown as a step
change in the anomaly score correlated with a timestamp, an atmospheric condition, a
frequency shift, or nothing at all. "Nothing at all" is also evidence — it means the
change was not environmental. It means someone made a decision.

Machine learning does not only make predators more effective. It makes defenders and
investigators more effective. That is what this project is.

---

## Part II — Architecture Principles

These principles govern every file, every agent, every track. They are not preferences.
Violation is a build failure.

### II.1 — The Forensic Rule

This system is forensic infrastructure. Fake data is evidence tampering.

If a physical device is not connected, the system renders a hard `[DISCONNECTED]` state.
It does not generate synthetic signals. It does not animate placeholders. It does not
fill buffers with sine waves to keep the visualization moving. It halts the affected
pipeline, logs exactly why, and waits for real hardware.

The word "mock" does not appear in production code, UI labels, or comments. Controls
that lack Rust backend wiring display `[UNWIRED]` — meaning the real thing exists and
the wire has not been run yet. `[UNWIRED]` is removed in the exact same commit the
wiring is completed. Never separately.

Test files are test artifacts. Any `.iq`, `.pcm`, or `.cf32` file located under `tests/`
or `examples/` is blocked from production ingestion by a hard assertion at the ingester
boundary. Attempting to pass a test file to a production ingester returns
`Err(BackendError::InvalidData("Test files must not be used in production"))` — not a
warning, a hard error.

### II.2 — The Hardware Abstraction Rule

Every algorithm references a trait, never a device. The C925e, the stereo camera, the
iPhone — these are all implementations of `VideoSource`. The Pluto+, the RTL-SDR, the
soundcard — these are all implementations of `SignalBackend`. The specific device appears
only in configuration and `Cargo.toml`. Never in algorithm code.

This is not just good architecture. It prevents agents from anchoring to specific hardware
and producing code that only works with one device. The Gaussian splatting pipeline does
not know what camera produced the frames. The WRF-GS model does not know whether the IQ
came from a Pluto+ or a file. The forensic corpus does not know what hardware wrote
observations into it. Every layer is separated from the hardware by a trait boundary.

### II.3 — The GPU Residency Principle

**Empirical basis**: During early development with a GTX 1070, agents accidentally used
the CPU build of CuPy. DTW, Levenshtein distance, wav2vec2, and several signal
processing chains ran in real time anyway — on the CPU. This is not a performance
problem. These algorithms are lightweight.

The conclusion this leads to is not "the CPU is fine." It is the opposite: if these
algorithms are so lightweight that they run real-time on a CPU even when accidentally
routed through a Python FFI layer on a 2016 GPU budget, then the reason to move them
to the GPU is not throughput. It is residency.

**The visualization is not a layer on top of the processed data. It is the processed
data.** An FFT output is a frequency-indexed array. The Emerald City color assignment is
a function of that array. The particle position is a function of the frequency and the
spatial estimate from the sensor array. When you run FFT on the CPU, copy the result to
Python, compute colors, copy again to the GPU buffer, you have moved the same data three
times through a path that has bandwidth limits, cache misses, and PCIe overhead at each
boundary. The data starts on the GPU (IQ samples arrive via DMA). The data ends on the
GPU (rendered as pixels). Every CPU stop in between is waste.

The same logic applies to DTW, Levenshtein, and wav2vec2. If they are lightweight enough
to run on a CPU in real time, they are lightweight enough to run as GPU compute shaders
with no perceptible latency — and when they run on the GPU, their output is already where
the rendering pipeline needs it. No copy. No boundary. No latency.

**The point cloud FFT problem** is a concrete instance of this principle. Standard GPU
FFT libraries (VkFFT, cuFFT, hipFFT) operate on 1D, 2D, or 3D regular grids. Point
clouds are unstructured — observations at arbitrary spatial positions, not on a lattice.
There is currently no standard GPU library that provides the equivalent of FFT for
unstructured point clouds: spatial frequency analysis, density estimation, nearest-
neighbor queries, and pressure gradient computation. We build it.

This is not a workaround. This is the system's primary GPU contribution. The kernels we
write for unstructured spatial signal analysis are the component that does not exist
anywhere else in this form.

**The division of labor**:
- CPU: hardware I/O, forensic corpus writes, control flow, Dorothy's LLM reasoning
- GPU: everything that touches signal values — FFT, wavelet, color assignment, spatial
  analysis, physics, scene representation, rendering, DTW, Levenshtein, wav2vec2

The GPU boundary is the DMA input from hardware. Data crosses into GPU memory once, via
DMA. It does not come back to the CPU until it exits as a corpus write or a rendered
frame. Not for intermediate processing. Not for "inspection." Not for logging — the
logging path reads the GPU buffer via mapped memory, not via copy.

**Architectural consequence**: any algorithm that takes signal values as input must have
a WGSL compute shader implementation, even if a CPU implementation also exists for
testing. The CPU implementation is the reference. The WGSL implementation is production.
These are maintained in parallel. When they disagree on output, the CPU implementation
is assumed correct and the WGSL implementation is debugged.

The parallelism that made the CPU version work so well — the multiple independent agent
loops processing different signal streams simultaneously — maps directly onto GPU
compute. Each stream is a dispatch. Each dispatch is independent. The GPU runs them
concurrently in hardware. The CPU ran them concurrently in software threads. The
architecture was always GPU-shaped. The GPU just does it better.

### II.4 — The Memory Security Rule

This system handles sensitive forensic data on hardware that emits RF. Sound cards,
GPUs, and RAM sticks have measurable electromagnetic emission profiles. An attacker with
appropriate equipment and proximity can read memory access patterns from RF emissions
(TEMPEST). The memory architecture must minimize this attack surface.

All large data structures use memory-mapped I/O via `memmap2` with `MAP_PRIVATE`
semantics during processing. Data is never copied through intermediate heap buffers when
a mapped slice will do. `clone()` on large structures requires a justification comment.
The forensic corpus is flushed to disk with `fsync` after every write — it is never held
in a heap-allocated buffer longer than one processing window.

`unsafe` blocks require a one-line justification comment explaining exactly why the
safe alternative is insufficient. `unsafe` used to work around borrow checker complaints
is never acceptable — that is always a design error. The borrow checker is correct.

### II.5 — The Idiomatic Rust Rule

This codebase is Rust. It is not C++ with Rust syntax. The following patterns are build
failures equivalent to `todo!()`:

Raw pointer arithmetic for performance without a documented TEMPEST or latency
justification. Manual memory management where ownership transfer achieves the same
result. `unsafe` blocks without justification comments. `clone()` on structures larger
than 128 bytes without a documented reason. Global mutable state outside of `AtomicU32`
and `AtomicU64` used for lock-free ring buffers. Blocking operations on async threads.
`std::thread::sleep` anywhere in a hot path.

Structure of Arrays layout for any buffer holding more than 10,000 elements. This is not
optional — it is required for cache-coherent GPU reads.

### II.6 — The 128-Byte Law

All structs crossing the CPU/GPU boundary are exactly 128 bytes — one RX 6700 XT
Infinity Cache line. Padding fields are named active heuristics, never `[u8; N]` dummies.
Every such struct has `const _: () = assert!(std::mem::size_of::<T>() == 128);`
immediately after its definition. If the struct drifts, the build fails.

### II.7 — The Wave64 Mandate

All WGSL compute shaders use `@workgroup_size(64, 1, 1)`. Never 32. Never 128. RDNA2
executes exactly 64-thread wavefronts. This is a hardware requirement, not a preference.

### II.8 — The Timestamp Rule

Hardware timestamps use `QueryPerformanceCounter` via `windows-sys`. Not
`std::time::Instant`. Not `SystemTime::now()`. The session epoch QPC is captured once at
process start. All subsequent timestamps are `(current_qpc - epoch_qpc) /
(freq / 1_000_000)` in microseconds. This is what ETW uses. This is what the forensic
corpus requires. This is what survives cross-examination.

### II.9 — The Variable Backend Rule

Every algorithm that produces or consumes a waveform is written against `SignalBackend`,
not against specific hardware. Three backends compile from day one:

`Backend::Audio` — 24-bit sound card via CPAL/WASAPI. Always the first test target.
`Backend::Pluto` — AD9363 via libiio. Second target after Audio is green.
`Backend::File` — write IQ/PCM to disk. Always runs as a side effect of every session.

No algorithm milestone is complete until verified against `Backend::Audio` and
`Backend::Pluto`. `Backend::File` output is generated automatically from every session
with no user action required. The file is the evidence.

### II.10 — The Pre-Flight Rule

Every agent writes this block at the top of its first new file:

```rust
// === PRE-FLIGHT ===
// Task:           [Track X, Milestone Y]
// Files read:     [list every file read before writing this one]
// Files in scope: [list every file this task may modify]
// Acceptance:     [verbatim from this document]
// Findings:       [relevant patterns observed in existing code]
// === END PRE-FLIGHT ===
```

### II.11 — The Proportional UI Rule

Hardcoded pixels are forbidden. Every dimension is expressed as a multiple or fraction
of a single root property: `property <length> unit-size: 16px;`. Macro-components
respect the Golden Ratio (Φ ≈ 1.618) in their aspect ratios. The UI scales to any
display — the Raspberry Pi CRT, a courtroom monitor, a phone — without rebuilding.

---

## Part III — The Unified Perceptual Scene

This is the system's primary output for human observers. It is not a dashboard. It is
not a spectrum analyzer. It is a scene — a 3D space in which every signal type exists
simultaneously as spatial geometry rendered in physically meaningful color.

### The Coordinate System

A standard RGB camera (any camera — C925e, stereo camera, iPhone, recorded file) produces
video frames. A monocular depth estimation model (Depth Anything V2, MiDaS, or DPT)
converts those frames into a dense 3D point cloud of the scene in real time on the GPU.
This point cloud is the spatial anchor. Everything else registers to it.

The room you can see is the coordinate system. The signals you cannot see float in that
same space. When you point to a location in the room and say "that's where it's coming
from," the system can query the RF field at that exact coordinate, retrieve its history,
and produce a timestamped record.

### The Signal Layers

**Layer 0 — Video point cloud**: The depth-reconstructed scene. Looks like the room.
Immediately legible to anyone. This is always on. It is the ground truth that everything
else is registered against.

**Layer 1 — Pose estimation**: Human bodies in the scene are detected and represented as
skeletal point clouds via MediaPipe or equivalent. When an RF anomaly is centered on a
human body and moves when that body moves, this layer makes that visible without
requiring any explanation.

**Layer 2 — Acoustic field**: The microphone array (or single microphone) produces a
pressure field. Via time-difference-of-arrival or beamforming, acoustic energy is
spatially localized and rendered as colored particles in the scene coordinate system.
Low frequencies appear as large, slow-moving volumes. High frequencies appear as small,
rapid structures.

**Layer 3 — Electromagnetic field**: RF observations from the RTL-SDR, Pluto+, or WiFi
CSI are rendered as Gaussian splats in the scene coordinate system. Color is assigned
by the Emerald City harmonic mapping (described in Part V). Phase coherence modulates
brightness — constructive interference is bright, destructive interference is dark. A
null is not absent. It appears as dark geometry. It is visible.

### The Color Language

All three signal layers use the same color system, derived from the Flutopedia
pitch-to-color standard. The anchor is octave equivalence: a frequency and its octave
multiples receive the same hue regardless of absolute value. This means 440 Hz (A4) and
2.4 GHz (WiFi channel 6, which folds to an A-equivalent octave) appear in the same
family of colors. The electromagnetic environment and the acoustic environment speak the
same color language.

Primary mappings follow the chromatic circle: C = red, D = orange, E = yellow, F = green,
G = cyan, A = blue, B = violet. Mixed frequencies produce mixed colors. A WiFi signal
overlapping with a specific audio frequency produces the additive mixture — a yellow where
red RF and green acoustic coincide.

The key implication for the investigation use case: if you are told "I feel this here,"
and the visualization shows a bright, saturated, anomalous color concentration at that
spatial location that does not correspond to any known device, that is evidence. It is
not a number on a screen. It is a thing floating in space that a person can look at and
point to.

### The Toggle System

Six toggles, accessible during a live demonstration without navigating menus:

`V` — Video point cloud on/off  
`P` — Pose estimation on/off  
`A` — Acoustic field on/off  
`R` — RF field on/off  
`E` — Environmental overlay on/off (temperature, humidity, GPS, weather)  
`T` — Timeline scrub mode (replay from corpus at any timestamp)

The demo flow: start with V only — the room, recognizable. Add P — the people in the
room appear as skeletal structures. Add A — the sound field becomes visible. Add R — the
RF environment appears. At each step the observer can process what they're seeing before
the next layer arrives. The final state is the complete picture.

---

## Part IV — Hardware Topology

```
┌─────────────────────────────────────────────────────────────────────┐
│                     MAIN PC (Windows 11)                            │
│   RX 6700 XT + Ryzen 7 5700X + 64GB RAM (SAM/ReBAR enabled)        │
│                                                                     │
│  GPU owns: point cloud processing, FFT, WRF-GS, pose estimation,   │
│            scene rendering, physics simulation, waveform synthesis  │
│  CPU owns: hardware I/O, forensic corpus writes, control flow,      │
│            LLM inference (Dorothy), MCP protocol                    │
└──────┬──────────────────┬───────────────────────┬───────────────────┘
       │ USB              │ USB-C or Ethernet      │ USB
       ▼                  ▼                        ▼
┌──────────────┐ ┌─────────────────────┐ ┌──────────────────────┐
│  Coral TPU   │ │  Pluto+ (AD9363)    │ │  Pico 2 (RP2350)     │
│  INT8 infer  │ │  12-bit TX/RX       │ │  ESN classifier      │
│  NF anomaly  │ │  70 MHz – 6 GHz     │ │  128-node INT8       │
│  calibration │ │  Onboard Linux ARM  │ │  Hardware timestamps │
│              │ │  libiio interface   │ │  Independent of PC   │
└──────────────┘ └─────────────────────┘ └──────────────────────┘

USB: Any VideoSource (C925e, stereo camera, depth camera — all same trait)
USB: RTL-SDR V4 (8-bit RX, 24–1766 MHz, GNSS capable at 1575.42 MHz)
I2C/USB: BME280 or equivalent (temperature, humidity, pressure)
WiFi: CSI extraction from existing router infrastructure (WiGrus methodology)
Future USB nodes: iPhone, additional cameras, additional SDRs
```

**RX 6700 XT**: 12GB VRAM, RDNA2. SAM/ReBAR must be enabled before any GPU buffer
work. Full 12GB VRAM addressable as contiguous BAR. Without SAM, only 256MB addressable
— the zero-copy pipeline breaks. Verify before Track G work, log to
`assets/hardware_gate.txt`.

**Pluto+**: Tethered Linux dev board, not a passive peripheral. ARM CPU runs onboard
inference. FPGA fabric available for future hardware FFT acceleration. 12-bit ADC/DAC —
practical SNR 50–60 dB in a real room.

**RTL-SDR V4**: 8-bit, 2.8 MSPS max. Primary use: spectrum monitoring, GNSS reception
(1575.42 MHz L1 GPS with patch antenna), environmental baseline capture. GNSS provides
independent time verification not dependent on NTP or PC clock — critical for forensic
timestamp corroboration.

**Pico 2**: Hardware timestamps from RP2350 hardware counter, independent of PC clock.
ESN classifier runs without the PC. When the PC clock and the Pico timestamp agree, the
timestamp is doubly verified. When they diverge, that divergence is itself logged.

---

## Part V — The Emerald City Color System

The Emerald City color system is not a visualization tool bolted onto signal data. It is
the signal data, rendered. The color assignment is computed once, on the GPU, as part of
the FFT output processing. The particle system reads the color directly from the GPU
buffer where the FFT result lives. There is no copy, no translation layer, no separate
"visualization pass."

### The Harmonic Mapping

Anchor: F4 (349.23 Hz) → hue bucket 5 (violet, matching Flutopedia standard).

For any frequency f:
`octave_position = log2(f / F4_hz) mod 12`

This maps every frequency — audio, RF, any band — to a position on the 12-step chromatic
circle. Frequencies that are exact octave multiples of each other receive identical hues.
A 2.4 GHz WiFi carrier and the audio note it folds to are the same color. The physical
relationship is visible.

The three HSL dimensions encode three independent signal properties:
- **Hue**: frequency (the harmonic mapping above)
- **Lightness**: phase coherence Γ (bright = constructive, dark = destructive)
- **Saturation**: inverse bandwidth (narrow-band = vivid, wideband = muted)

Phase coherence: `Γ(r) = |ΣᵢEᵢ(r)| / Σᵢ|Eᵢ(r)|`

A WiFi null (standing wave zero) appears as dark violet. It is present in the scene. It
has a location. It is not absent — it is the dark geometry of destructive interference.
That geometry is evidence.

### The Acoustic Heterodyne Layer

For each dominant FFT bin at frequency f_rf:
`f_audio = f_rf / 2^N` where N makes f_audio land in 20–1000 Hz.

This is the octave-folding computation — identical to the hue assignment, routed to a
second output: a synthesized audio tone played through `Backend::Audio`. Multiple bins
produce a chord. The RF environment becomes audible as the harmonic skeleton the color
mapper is already drawing.

`Backend::File` writes the synthesized PCM continuously as a QPC-timestamped session
file. This is the audio exhibit — a recording of what the RF environment "sounded like"
at a specific time. It is automatically produced. It requires no user action.

---

## Part VI — The Forensic Corpus

The forensic corpus is the system's primary output. Everything else serves it or reads
from it. Dorothy reads it. The visualization reads it. The corpus does not read anything.

### Structure

Every observation is a `FieldParticle` (128 bytes, defined in Part VII) written to
`databases/forensic_logs/events.jsonl` and simultaneously to a binary corpus file at
`databases/forensic_logs/corpus_YYYYMMDD.bin`. The binary file is the primary artifact.
The JSONL is the human-readable duplicate.

Every write is immediately followed by `fsync`. The file is opened with `O_DSYNC` on
platforms that support it. The corpus does not buffer.

### Chain of Custody

Immediately after capture, a SHA-256 hash is computed for each IQ capture block and
embedded in the `FieldParticle` metadata. The hash covers the raw samples plus the QPC
timestamp plus the device serial number or identifier. This follows C2PA 2.2 standards
for content provenance.

The POLE data model structures all forensic evidence:
- **Person**: pose-estimated human body present in scene at timestamp
- **Object**: identified signal source (device, frequency, spatial origin)
- **Location**: GPS-correlated spatial position (from GNSS-SDR or static fix)
- **Event**: timestamped observation linking Person, Object, and Location

These entities live as nodes in Neo4j with typed relationships. Every `FieldParticle` in
the binary corpus has a corresponding Neo4j node ID. The Qdrant vector store holds
embeddings of signal patterns for similarity search — "find me every observation that
looks like this one." The QdrantNeo4j retriever links similarity results to their full
forensic context.

### Environmental Correlation

Every `FieldParticle` is correlated with the environmental state at capture time:

- Temperature, humidity, barometric pressure (BME280 sensor or weather API)
- GPS position and satellite count (GNSS-SDR via RTL-SDR at 1575.42 MHz)
- UTC time from GNSS (independent time source, corroborates QPC timestamp)
- Weather API snapshot (NOAA free API, cached locally, not a dependency)
- Propagation adjustment: Double-Debye permittivity correction for current humidity

The sudden change in your environment — the moment the harassment reduced — would appear
as a step change in anomaly score. The environmental record would show whether that step
change correlated with temperature, humidity, a GNSS timestamp boundary, or nothing at
all. "Nothing at all" means the change was not environmental. It means a human decision
was made. That is the finding.

---

## Part VII — Core Data Structures

### FieldParticle (128 bytes, CPU/GPU boundary)

```rust
#[repr(C)]
pub struct FieldParticle {
    pub timestamp_us:               u64,        //  8  QPC microseconds from session epoch
    pub freq_hz:                    f64,        //  8  center frequency of observation
    pub energy:                     f32,        //  4  normalized 0.0–1.0
    pub phase_coherence:            f32,        //  4  Γ: 0.0=null, 1.0=constructive
    pub position_xyz:               [f32; 3],   // 12  spatial estimate, meters
    pub material_id:                u8,         //  1  octave bucket 0–11 (hue class)
    pub source:                     u8,         //  1  0=AudioHost,1=PlutoOnboard,
                                                //     2=HostProcessed,3=Pico,4=RTL,5=CSI
    pub layer:                      u8,         //  1  0=RF,1=Acoustic,2=Video,3=Environmental
    pub gnss_fix:                   u8,         //  1  satellite count, 0=no fix
    pub doppler_shift:              f32,        //  4  radial velocity estimate
    pub phase_velocity:             f32,        //  4  wavefront speed estimate
    pub scattering_cross_section:   f32,        //  4  effective scatter area
    pub bandwidth_hz:               f32,        //  4  spectral width
    pub anomaly_score:              f32,        //  4  Coral NF output; 0.0 if unavailable
    pub temperature_c:              f16,        //  2  environmental at capture time
    pub humidity_pct:               f16,        //  2  environmental at capture time
    pub motif_hint:                 u8,         //  1  ESN classification; 255=unknown
    pub corpus_hash:                [u8; 7],    //  7  first 7 bytes of SHA-256 of raw block
    pub embedding:                  [f32; 14],  // 56  first 14 dims of 128-D Mamba latent
}
// 8+8+4+4+12+1+1+1+1+4+4+4+4+4+2+2+1+7+56 = 128 bytes
const _: () = assert!(std::mem::size_of::<FieldParticle>() == 128);
```

### AetherParticle (128 bytes, GPU physics particle)

```rust
#[repr(C)]
pub struct AetherParticle {
    pub position:                   [f32; 3],   // 12  world space
    pub velocity:                   [f32; 3],   // 12  meters/second
    pub color_hsl:                  [f32; 3],   // 12  hue/lightness/saturation
    pub mass:                       f32,        //  4  proportional to energy
    pub lifetime:                   f32,        //  4  seconds remaining
    pub phase_coherence:            f32,        //  4  Γ at this position
    pub layer_flags:                u32,        //  4  bitmask: which layers contribute
    pub material_id:                u8,         //  1  octave bucket
    pub _pad0:                      [u8; 3],    //  3  alignment
    pub doppler_shift:              f32,        //  4  pre-computed heuristic
    pub pressure_gradient:          f32,        //  4  |∇P_SPH| for haptic LF channel
    pub rf_roughness:               f32,        //  4  α_RF for haptic HF channel
    pub scattering_cross_section:   f32,        //  4  pre-computed heuristic
    pub embedding_slice:            [f32; 12],  // 48  Mamba latent summary
    pub _pad1:                      [u8; 4],    //  4  alignment to 128
}
const _: () = assert!(std::mem::size_of::<AetherParticle>() == 128);
```

### HeterodynePayload (128 bytes, three-sense delivery)

```rust
#[repr(C)]
pub struct HeterodynePayload {
    pub timestamp_us:               u64,        //  8
    pub f_tactile_lf_hz:            f32,        //  4  haptic LF channel (< 80 Hz)
    pub f_tactile_hf_hz:            f32,        //  4  haptic HF channel (80–300 Hz)
    pub f_audio_hz:                 f32,        //  4  heterodyned tone for hearing
    pub audio_amplitude:            f32,        //  4  0.0–1.0
    pub motif_token:                u32,        //  4  Chronos motif ID
    pub motif_phase:                u8,         //  1
    pub motif_phase_total:          u8,         //  1
    pub _pad0:                      [u8; 2],    //  2
    pub motif_confidence:           f32,        //  4
    pub next_event_eta_secs:        f32,        //  4
    pub anomaly_score:              f32,        //  4
    pub phase_coherence:            f32,        //  4
    pub position_xyz:               [f32; 3],   // 12
    pub color_hsl:                  [f32; 3],   // 12
    pub embedding_slice:            [f32; 16],  // 64
}
const _: () = assert!(std::mem::size_of::<HeterodynePayload>() == 128);
```

---

## Part VIII — GPU-Native Signal Processing

The central architectural insight: **the visualization is the processed data**. The FFT
output is a frequency-indexed array. The color assignment is a lookup on that array. The
particle position is a function of the frequency and the spatial estimate. None of this
requires leaving the GPU.

### The GPU Processing Chain

```
Camera frame → GPU (DMA, zero copy)
    → Depth estimation (Depth Anything V2 compute shader)
    → Point cloud reconstruction (depth × camera intrinsics)
    → Pose estimation (MediaPipe GPU delegate)
    → Video layer particles (registered to scene coordinate system)

Microphone samples → GPU (DMA via CPAL zero-copy buffer)
    → FFT (VkFFT, 64-thread workgroups)
    → Beamforming / TDOA spatial localization
    → Phase coherence Γ per bin
    → Acoustic layer particles with color from Emerald City

RTL-SDR / Pluto+ IQ → GPU (DMA via libiio zero-copy)
    → FFT (VkFFT)
    → WRF-GS Gaussian parameter update
    → Phase coherence Γ per bin
    → RF layer particles with color from Emerald City

All layers → Unified scene coordinate system
    → SPH density pass (Kogge-Stone prefix scan, 64-thread workgroups)
    → PBD constraint solve
    → Compute-to-indirect draw (zero CPU draw call overhead)
    → Render (wgpu, 60 FPS)
```

### The Custom GPU Spatial Analysis Kernels

No existing GPU library provides the equivalent of FFT for unstructured point clouds.
Standard libraries (VkFFT, cuFFT, hipFFT) require regular grids. Signal observations
arrive at arbitrary spatial positions. We build the missing primitives as first-party
WGSL compute shaders, living in `src/cyclone/spatial_kernels/`.

These are not workarounds. They are the system's primary original GPU contribution.

**Kernel 1 — Spatial Hash Construction** (`spatial_hash.wgsl`)
Kogge-Stone parallel prefix scan. O(log N) passes in a single 64-thread workgroup.
LDS only — no global atomics, no inter-workgroup synchronization.
Output: sorted index array over the point cloud, partitioned by spatial cell.
Foundation for all subsequent O(1) and O(k) spatial queries.

**Kernel 2 — SPH Density Estimation** (`sph_density.wgsl`)
Müller 2003 poly6 smoothing kernel on the spatial hash output.
Computes density ρ and pressure gradient |∇P| at each particle position.
|∇P| feeds the haptic LF channel directly from GPU memory. No CPU readback.

**Kernel 3 — Nearest-Neighbor Query** (`knn_query.wgsl`)
O(1) lookup after spatial hash. Returns k nearest particles to any query position.
Used by: WRF-GS Gaussian parameter update, TimeGNN edge construction,
anomaly localization, scene coordinate registration of new signal observations.

**Kernel 4 — Gaussian Parameter Update** (`gaussian_update.wgsl`)
The "point cloud FFT" operation — the bridge between spectral domain and spatial
Gaussian representation. For each FFT bin: find spatial estimate from TDOA/RSSI
correlation → locate nearest Gaussian via Kernel 3 → update amplitude from bin
magnitude, phase from bin angle, position delta from TDOA gradient, covariance
from bin bandwidth. This is what makes the visualization the data.

**Kernel 5 — DTW on GPU** (`dtw.wgsl`)
The CPU cupy accident proved DTW is lightweight enough for real-time CPU execution.
On GPU, batched against the full template library, it becomes O(1) amortized per
query. Anti-diagonal DP wavefront fills naturally across 64-thread workgroups along
the wavefront dimension. Used by Wizard gesture matching and TimeGNN motif comparison.

**Kernel 6 — Levenshtein on GPU** (`levenshtein.wgsl`)
Edit distance for motif sequence comparison. Same wavefront parallelism as DTW.
Template library batch evaluation. CPU reference in `tests/levenshtein_reference.rs`.

**wav2vec2 on GPU**: The CPU version runs real-time. The GPU version batches across
multiple sliding windows simultaneously. Dispatch once per audio frame, read the
token sequence from the output buffer. The token sequence is already on the GPU where
TimeGNN needs it. No copy.

**CPU-GPU contract for all six kernels**: Every kernel has a CPU reference
implementation. Nightly CI runs both against identical test data and asserts numerical
equivalence within f32 tolerance (1e-5). When they diverge, the WGSL is wrong.
When the GPU-native point cloud processing ecosystem matures to provide these
operations, we switch. Until then, we own this code and know exactly what it does.

---

## Part IX — The Signal Processing Backend

### Mamba (UnifiedFieldMamba)

Selective state-space model. Linear-time sequence modeling O(N) versus Transformer O(N²).
Input-dependent selection mechanism — the Delta (Δ) parameter is computed per token,
acting as a gatekeeper that determines the importance of each input to the hidden state.

The key property for this use case: Mamba maintains a compressed representation of the
environment's entire observable history in a fixed-size hidden state. It does not store
the raw history. This means anomaly detection improves over time without linearly
increasing memory. The model learns what "normal" looks like for this specific
environment and flags deviations.

Discretization via Zero-Order Hold: Ā = exp(ΔA), B̄ = (ΔA)⁻¹(exp(ΔA) - I)ΔB.
The 128-D latent embedding produced by Mamba is the universal data currency — it feeds
the WRF-GS splat color, the TimeGNN graph nodes, the edge filter classifiers, and the
first 14 dimensions live in the `FieldParticle.embedding` field.

**Hardware target**: RX 6700 XT via Burn + wgpu. Kernel fusion, parallel scans.
5x throughput over equivalent Transformer on long sequences.

### TimeGNN

Graph neural network operating on the stream of FieldParticle embeddings. Nodes are
signal observations. Edges are temporal and spatial proximity relationships. Temperature
τ controls the density of edges — low τ connects only tight clusters (anomaly detection
mode), high τ connects broad relationships (exploration mode).

Motifs are named patterns extracted from the graph: recurring subgraph structures that
appear with statistical regularity above a silhouette score threshold of 0.6. Names are
adjective-noun pairs (GHOST, SPARKLE) — not UUIDs. The motif name is human-memorable
because it may need to be communicated verbally.

### LNN (Closed-form Continuous-time)

`dx/dt = -x/τ + f(x, input, t)·Δt`

τ is a learned time constant per node. Δt comes from actual `timestamp_us` differences
— not assumed uniform. This is the critical property: the LNN handles dropped packets,
variable buffer sizes, and the simultaneous tracking of microsecond RF bursts and
12-hour thermal cycles without any special casing. The physics of variable-rate time is
baked into the architecture.

### Normalizing Flow (Coral TPU, INT8)

Learns the explicit probability density of the empty room's baseline RF/acoustic noise.
Anomaly score = log probability under the learned density. A new signal that is
improbable under the empty-room model is anomalous. This is mathematically principled
anomaly detection — not a threshold, not a heuristic, a probability.

The empty-room model is the "before" state. If the harassment stops and you run the
system for 24 hours, the model updates. If you can document the "before" corpus and the
"after" corpus, the Normalizing Flow gives you a quantitative measure of how different
they are. That number is evidence.

### Echo State Network (Pico 2, RP2350, no_std)

128-node reservoir. Fixed INT8 random weight matrix (16KB). Readout weights trained
offline on the main PC, deployed as a const array. Classifies each 1ms window
independently. Hardware timestamps from RP2350 counter, independent of PC clock.

The Pico's role: produce the first-pass classification faster than the PC can process.
When the Pico and the PC agree on a classification, confidence is high. When they
disagree, that disagreement is itself a signal worth logging.

### LFM 2.5 (Dorothy's reasoning backbone)

1.2B parameter hybrid architecture: convolutional blocks + grouped-query attention.
125K token context window. Chain-of-thought reasoning output before final response —
critical for forensic workflows where the reasoning process is as important as the
conclusion. Runs on-device. Does not require cloud connectivity. Forensic data never
leaves the local system.

Dorothy uses LFM 2.5 to read the forensic corpus, identify patterns, generate
hypotheses, propose cleansing strategies, and produce human-readable summaries suitable
for non-technical audiences including law enforcement and legal counsel.

---

## Part X — Agent Roster

**Dorothy** — The Mind
Electromagnetic field analysis, WRF-GS synthesis, spectrum reasoning. Reads the forensic
corpus. Produces hypotheses and cleansing strategies. Uses LFM 2.5 as reasoning backbone.
Writes to Joplin journal. Never writes synthetic data to the corpus.

**Glinda** — The Memory Engine
Multi-modal episodic memory, semantic graph, physics simulation substrate. Three tiers:
sensory buffer (60s ring buffer, lock-free), episodic records (PostgreSQL + Qdrant),
semantic graph (Neo4j, POLE model). The corpus feeds Glinda. Glinda does not generate
observations — it indexes them.

**Scarecrow** — The Brain Interface
EEG/BCI, neural signal decoding, cognitive state monitoring. CNN-GAN for visual
reconstruction, LSTM/Transformer for auditory decoding, emotion classification. Correlates
neural state with RF environment state. Correlation between physiological response and
signal anomaly is a category of evidence.

**Tin Man** — The Voice
EMG silent speech, gestural control. ResNet-Conformer architecture. MONA LLM
disambiguation. The interface for when you cannot speak aloud.

**Lion** — The Defender
Autonomous interference detection and cleansing. Raspberry Pi + Coral. High autonomy
with cognitive oversight. Logs every intervention — what it did, when, what effect it
measured.

**Crystal Ball** — The Inference Engine
Shared ML model serving. ONNX Runtime + MIGraphX for AMD GPU. TensorFlow Lite for
Coral. Model versioning and A/B testing. All agents share this substrate.

**Wizard** — The Controller
Gestural input, spatial wave manipulation, haptic feedback. IMU fusion, Extended Kalman
Filter, DTW gesture recognition. The physical interface for direct field interaction.

**Oz** — The Visualizer
The unified perceptual scene renderer. wgpu + bevy_gaussian_splatting. 60 FPS. All
signal layers. Toggle system. The exhibit.

**Emerald City** — The Translator
Frequency-to-color mapping, spatial correlation, phase coherence computation. Runs
entirely on GPU as part of the FFT-to-particle pipeline. Not a separate processing step.

**Cyclone** — The Signal Engine
Universal signal processing substrate. Lock-free ring buffers, VkFFT, Daubechies
wavelets, CSI preprocessing, point cloud FFT kernels. The GPU compute backbone.

**Brick Road** — The Hardware Abstraction
`VideoSource`, `SignalBackend`, `EnvironmentSource` traits. Every device is an
implementation. Hot-swapping, device discovery, calibration management.

**Kansas** — The Interface
ratatui TUI for system state and diagnostics. The operational interface when Oz is not
running. CRT aesthetic on edge devices.

**Toto** — The Edge Orchestrator
Raspberry Pi node coordinator. Manages Coral, Pico 2, Pluto+. Future: federated learning
coordination, drone deployment.

---

## Part XI — Track Specifications

### Phase 0 — Foundation

**0-A: FieldParticle + SignalIngester**
Files: `src/ml/field_particle.rs`, `src/dispatch/signal_ingester.rs`,
`src/dispatch/audio_ingester.rs`, `src/dispatch/rf_ingester.rs`,
`src/dispatch/video_ingester.rs`, `src/dispatch/environment_ingester.rs`.
Doc-tests: `freq_to_material_id(440.0)` → 9, `freq_to_material_id(880.0)` → 9,
`freq_to_material_id(349.23)` → 5, `freq_to_material_id(2_400_000_000.0)` → document.
Blocks: everything.

**0-B: Design Language Tokens**
`ui/tokens.slint`, `assets/SKILL-SLINT-MD3.md`. `unit-size: 16px` root property defined.
Golden Ratio macro-component aspects. `slint-viewer ui/tokens.slint` zero errors.
Blocks: Tracks E, F, 0-D.

**0-C: SAM Hardware Gate**
Verify RX 6700 XT BAR size ~12GB via wgpu adapter info. Log to `assets/hardware_gate.txt`.
Enable in BIOS if needed (AMD CBS → NBIO → Above 4G Decoding + ReBAR Support).
Blocks: Tracks G, H.

**0-D: Hardware Configuration Applet**
`ui/hardware.slint`. Repository cleanup first — delete all examples except `toto` and
`hardware`, all tests, all `.slint` files except `toto.slint` and `tokens.slint`.
Material Design 3 hardware cards for VideoSource backends, RTL-SDR, Pluto+, Soundcard.
Full radio tuner per device. CW, sinc-filtered tone, W-OFDM, file IQ waveform modes.
`unit-size` proportional layout, Golden Ratio card aspects, `[UNWIRED]` not `[MOCK]`.
Physical TX test: Pluto+ to 1 MHz CW, measurable in SDR++.
Physical audio test: Soundcard to 440 Hz sine, audible through speakers.
Backend::File auto-write for every session. QPC timestamps. Test path blocking assertion.
Blocks: nothing directly — but proves the hardware interface before agents need it.

---

### Track G0 — Video Point Cloud (NEW — precedes RF Gaussians)

**Depends on**: 0-A, 0-C  
**Goal**: The spatial anchor for the unified perceptual scene. Must exist before the RF
and acoustic layers can be registered to a coordinate system a human can understand.

**G0-1 — Monocular depth estimation**
`VideoSource` trait backend for any RGB camera. Depth Anything V2 or MiDaS running as
a Crystal Ball registered model. GPU compute shader converts depth map to point cloud.
No depth camera required — any RGB video source produces a 3D reconstruction.
Acceptance: `cargo run --example oz_preview` shows a live point cloud of the room from
the C925e at 30+ FPS. Room geometry is recognizable. No crash when camera is unplugged
— renders `[DISCONNECTED]` state.

**G0-2 — Pose estimation integration**
MediaPipe Pose running as Crystal Ball registered model. Skeletal keypoints extracted
and rendered as colored point structures in scene coordinate system. Human bodies become
spatial objects.
Acceptance: Person walking through frame produces visible skeletal structure moving
through the point cloud scene. Skeletal structure disappears cleanly when person leaves
frame.

**G0-3 — Scene coordinate system establishment**
SLAM-lite via feature tracking or static calibration. All subsequent signal layers
register to this coordinate system.
Acceptance: Mark a known physical location (corner of room). RF observations at that
location appear at the correct position in the point cloud scene. Position error < 30 cm.

---

### Track A — Mamba Inference Loop

**Depends on**: 0-A. **Independent of**: all UI tracks, ROCm, Vulkan.

**A1 — Dispatch loop wiring**
Remove the 9× audio repeat hack. Wire: `AudioIngester::ingest()` → accumulate 4096 unique
samples → `Mamba::forward()` → `project_latent_to_waveshape()` → Drive/Fold/Asym.
`project_latent_to_waveshape()` uses three learned linear projections from the 128-D
latent to three scalars. No synthetic fallback. If audio device not found: halt and log.
Acceptance: 60 seconds logged. Drive ≠ Fold ≠ Asym. Values change over time.

**A2 — RF ingester integration**
`RFIngester` alongside `AudioIngester`. Pluto+ optional — can read pre-recorded `.iq`
file (raw interleaved f32 complex, little-endian, 8 bytes per sample, no header). File
size must be divisible by 8 or ingester halts with error.
Acceptance: material_id 0–4 (audio) and 5–11 (RF) both appear in same 10-second window.

**A3 — Video ingester integration**
`VideoIngester` feeding depth point cloud observations into the FieldParticle stream.
Video-sourced particles use `layer: 2`. Material_id derived from pixel color temperature
for visual coherence.
Acceptance: Camera observations appear in forensic corpus with correct layer flag and
scene coordinate positions.

**A-ENV — Environmental ingester**
`EnvironmentIngester` produces `FieldParticle` with `layer: 3` from BME280 sensor or
NOAA weather API. GNSS-SDR position fix from RTL-SDR at 1575.42 MHz populates
`gnss_fix` field. UTC from GNSS corroborates QPC timestamp.
Acceptance: Temperature, humidity, pressure appear in corpus. GNSS fix count logged.
When BME280 not connected: weather API fallback. When neither available: halt env
ingester, log reason, all other ingesters continue.

**A-HET — Heterodyning to acoustic base ratios**
f_audio = f_rf / 2^N landing in 20–1000 Hz. Multiple bins = chord. `Backend::Audio`
plays in real time. `Backend::File` writes timestamped PCM session file automatically.
Acceptance: Known 2.4 GHz WiFi AP produces correct folded tone. AP off = tone disappears.
PCM session file written to `assets/session_YYYYMMDDTHHMMSS.pcm`.

**A-EC — Phase Coherence (Emerald City)**
Γ(r) = |ΣᵢEᵢ(r)| / Σᵢ|Eᵢ(r)| per-bin. HSL: hue = frequency, lightness = Γ,
saturation = inverse bandwidth. Feeds G-SPH2 as RF repulsion field.
Acceptance: Known standing wave produces dark band at λ/2 null distance within 5%.

**A-WOFDM — Wavelet OFDM Synthesis**
WGSL compute shader: IDWT synthesis, Daubechies compact support, no guard intervals,
≥20% symbol density improvement over standard OFDM. Prove on Backend::Audio first.
Then Backend::Pluto. Backend::File always.
Acceptance: Audio loopback shows ≥20% improvement. Pluto loopback matches within noise.

**A3-EDGE — Edge filter deployment**
Coral TPU: INT8 Normalizing Flow for anomaly calibration. `AtomicU32` shared score.
Pico 2: 128-node ESN, ≥1kHz classification, hardware timestamps.
Pluto+ ARM: cross-compiled ESN/NF, source field `PlutoOnboard` vs `HostProcessed`.
All three degrade gracefully on unplug. No panic. No synthetic fallback.

---

### Track B — TimeGNN + LNN

**Depends on**: 0-A, A1.

**B1**: Real corpus from `databases/forensic_logs/events.jsonl`. Halt if empty. Silhouette
≥ 0.6 gate. 10 epochs, checkpoint, at least one "rejected: score X.XX" in log.

**B2**: Hot-swap τ, prediction horizon, attention window via TOML watcher. No restart.
τ change 0.14→0.80 takes effect within one epoch.

**B3**: `MotifEvent { name, phase, phase_total, confidence, next_event_eta_secs, freq_hz }`.
Adjective-noun names. Real model in same session as mock stream proof.

**B4**: NT-Xent loss as `AtomicU32`, lock-free ring buffer 120 values. Loss trends 2.1→0.05.

**B5**: CfC LNN via Burn crate. `dx/dt = -x/τ + f(x,input,t)·Δt`. Actual elapsed time
from `timestamp_us` differences. 30% dropped steps: LNN ≤15% above baseline.
Fixed-step RNN degrades significantly — document both.

---

### Track C — Glinda Memory Engine

**C1**: PostgreSQL + Qdrant, MCP tools, `query_semantic("WiFi interference")` < 100ms.
**C2**: Lock-free ring buffer, 60s FieldParticles, 192kHz × 512 particles zero dropped.
**C3**: Neo4j POLE model, typed edges, ≥3 entity types, ≥2 relationship types after 24h.

---

### Track D — Dorothy Cognitive Loop

**D1**: LangGraph Wake→Observe→Compare→Analyze→Hypothesize→Document→Sleep. 3h unattended,
3 notes with real Glinda observation IDs.
**D2**: Weekly synthesis note from 5 notes sharing common tag.
**D3**: MCP interface. `get_dorothy_opinion` cites ≥1 journal entry by date.

---

### Track E — Toto Widget

**E1**: Static with `[UNWIRED]` badges. Three zones. Wave color cycles. Opens <3s.
Zero `todo!()`.
**E2**: Live data from Track A. DWM Acrylic blur. Wave color responds to real frequency.
**E3**: WASM build. Loads in Chrome/Edge.

---

### Track F — Chronos Widget

**F1**: Static with `[UNWIRED]`. τ slider visible. Graph teal→violet→red.
**F2**: τ slider live (0.05→2.0 logarithmic). Edge density changes within one frame.
**F3**: Settings flyout from right. Freeze toggle disables sibling controls.
**F4**: Live from Track B. τ change affects training within one epoch.

---

### Track G — WRF-GS Scene + Physics + PINN

**Depends on**: A1, G0-1 (video point cloud as coordinate anchor), 0-C.

**G2-RDNA2**: `@workgroup_size(64,1,1)` all shaders. SoA layout >10k elements.
128-byte boundary structs everywhere.

**G1**: 1000 Gaussians, 60 FPS, camera orbits. `oz_preview` example.

**G2**: 128-D embedding splats, 10k Gaussians, ≥30 FPS, WiFi produces violet cluster.

**G3**: Daubechies 6-level wavelet per Gaussian. RF reflection AND acoustic absorption
distinct at each scale. Single-scale comparison documented.

**G-SPH1**: SPH density, Kogge-Stone prefix scan, 1M particles ≤2ms, 1% analytical error.
**G-SPH2**: PBD solve, stable at 16.67ms dt, WiFi AP causes visible particle clustering.
**G-SPH3**: Compute-to-indirect, DrawIndirectArgs without CPU readback, ≤2ms added.

**G-RB1**: Complex Fresnel, ITU-R P.2040 material library, ±1dB dry concrete.
**G-RB2**: RF-GGX α_RF = σ_surface/λ_RF. Near-specular 2.4GHz, diffuse 60GHz.
**G-RB3**: Double-Debye wetness. Dry vs wet wood measurable difference.

**G4**: PINN loss wrapper, Maxwell + acoustic wave equation constraints. Impossible
configuration causes ≥100x loss increase and halts.

**G5**: BVH, <5ms build 10k Gaussians, <0.1ms nearest-Gaussian query.

---

### Track H — Ray Tracing TX + Modulation

**Depends on**: G5, A1, B2.

**H1**: N rays through BVH, multipath delay spread 1–50 ns histogram.
**H2**: Phase accumulation, known geometry within 5 degrees.
**H3-DPC1**: Tomlinson-Harashima Precoding, ≥6dB SNR improvement vs naive.
**H3-DPC2**: Room-as-codec null, ≥15dB null depth at 10× lower power.
**H-QAM1**: Constellation profiling 64→4096, find EVM <-30dB ceiling.
**H-QAM2**: Three-layer symbol packing, all three decoders recover correctly.
**H-FRAC1**: GPU IDWT fractal (Daubechies-4 macro + Daubechies-8 micro). Audio proof
first, then Pluto+. Micro layer invisible to 6-bit decoder.
**H4**: Pico 2 TX trigger, <50μs jitter. Post-stabilization only.

---

### Track HA — Haptics (600Hz)

**Depends on**: G-SPH2, G-RB3.

**HA1**: 600Hz PBD solve, dedicated CPU core, 3-buffer async ring, no render impact.
**HA2**: LF/HF bifurcation. Pacinian corpuscles. Blind A/B test distinguishes surfaces.
**HA3**: Stochastic resonance. Metal = clean tone. Concrete = noisy envelope.

---

### Track I — Biometric Cloak (Capstone)

**Depends on**: All A–HA at final milestones, NixOS migration complete.

**I1**: E(3)-equivariant network, >90% "person present" from RF alone.
**I2**: Score-Based Diffusion, ≥15dB CSI reduction vs no cloak.
**I3**: LNN adaptive cloak, stable throughout slow room crossing.
**I4**: Optical cloak — conceptual only, legal review required, do not implement.

---

### Phase J — Post-Track-I Extensions

J1 RF Proprioception, J2 Sub-Nyquist Spectral Retina, J3 RF-Texture-Smell,
J4 Ambient Backscatter, J5 RIS Haptic Fields, J6 Differentiable Calibration,
J7 Field Compass UX.

### Phase K — Impulse Radio (Legally Gated)

K1 rpitx W-OFDM PoC, K2 Impulse ranging, K3 Multi-node fractal mesh.

---

## Part XII — The Unified Demo Flow

This is the sequence for showing the system to any non-technical observer.

1. Open Oz. Toggle V only. Show the room as a point cloud. "This is what the camera sees,
   reconstructed in 3D." The observer recognizes the room. Trust established.

2. Toggle P on. Skeletal structures appear over the people in the room. "This is pose
   estimation — the system knows where people are." The observer sees themselves.

3. Toggle A on. The acoustic field appears as colored volumes. "This is the sound in the
   room, visualized. Notice the color — red here, green there. These colors mean specific
   frequencies." Demonstrate by speaking — the acoustic field responds.

4. Toggle R on. The RF field appears. "This is the electromagnetic environment — WiFi,
   cellular, everything else. Same color language as the sound. Notice anything unusual?"
   Point to anomalies.

5. Press T. Scrub the timeline back to a documented event. "Here is what this room looked
   like at 3:47 AM on [date]. Here is the anomaly. Here is the timestamp. Here is the
   atmospheric condition at that time. Here is the hash of the raw data file."

6. Export the exhibit package: the timestamped corpus files, the GNSS-corroborated
   timestamps, the SHA-256 hashes, the Neo4j subgraph of the event, the rendered
   video of the visualization. This is the packet that goes to the lawyer.

---

## Part XIII — Technology Stack

| Track | Language | Key Libraries | GPU |
|-------|----------|---------------|-----|
| 0-A | Rust | rustfft, bytemuck, memmap2 | No |
| 0-B | Slint DSL | — | No |
| 0-D | Rust + Slint | slint 1.15, CPAL, windows-sys | No |
| G0 | Rust | wgpu 28, mediapipe (via PyO3 or native) | Yes |
| A | Rust | burn 0.21, wgpu 28 | Yes |
| A3 Pluto | Python or Rust armhf | burn-no-std or numpy | No |
| A3 Pico | Rust Embassy no_std | embassy-usb | No |
| B | Rust | burn 0.21, petgraph | Yes |
| C | Rust + Go | genkit-go, qdrant-client, tokio-postgres | No |
| D | Python | langgraph, LFM 2.5, joplin-api | No |
| E | Rust + Slint | slint 1.15, windows-sys | No |
| F | Rust + Slint | slint 1.15 | No |
| G | Rust | wgpu 28, WGSL | Yes DX12 |
| H | Rust | wgpu 28, nalgebra | Yes DX12 |
| HA | Rust | CPAL for VCA, wgpu | Yes |
| I | Rust + Python | all of above | Yes |

**PyO3 policy**: Python is acceptable for Dorothy (LangGraph, LFM 2.5), Glinda MCP
wrapper (genkit-go), and any model training pipeline. Python never appears in the
real-time signal processing path. PyO3 is the bridge when a Python library provides
something not available in Rust (specific model implementations, specific data science
tooling). The hot path is always Rust + WGSL.

**Interop hierarchy**: Rust owns the runtime. Python owns the AI reasoning layer.
TypeScript is acceptable for any web-based visualization or reporting interface. C/C++
is acceptable only for wrapping existing C libraries (libiio, rtlsdr) and must be
isolated behind a Rust `unsafe` wrapper with a documented justification.

---

## Part XIV — ROCm / NixOS Migration Gate

Do not migrate until:
- [ ] A1: Drive/Fold/Asym confirmed from real particle input
- [ ] E2: Toto widget live on Windows 11 with real data
- [ ] G1: WRF-GS render at 60 FPS on Windows 11 DX12
- [ ] G0-1: Video point cloud live from C925e
- [ ] B1: TimeGNN checkpoint saves from real corpus
- [ ] 72 hours continuous operation without crash

Migration adds: ROCm HIP backend, Vulkan ray tracing, KWin compositor blur.
Migration changes nothing: APIs, Slint, track structure, libiio interface.
DX12 → Vulkan is a backend swap, not a rewrite.

---

## Part XV — Neural Architecture Reference

| Model | Track | Purpose | Target |
|-------|-------|---------|--------|
| UnifiedFieldMamba | A | 128-D embeddings, anomaly history | GPU |
| TimeGNN | B | Temporal pattern graph, named motifs | GPU |
| LNN (CfC) via Burn | B5 | Variable-rate temporal dynamics | GPU |
| Normalizing Flow | A3, I2 | Anomaly probability; empty-room baseline | Coral TPU |
| Echo State Network | A3 | Fast first-pass, hardware timestamps | Pico 2 + Pluto+ ARM |
| NT-Xent Contrastive | B | Motif similarity, temperature τ | GPU |
| PINN | G4, H3 | Maxwell + acoustic constraints on TX | GPU |
| Depth Anything V2 | G0 | Monocular depth → point cloud | GPU |
| MediaPipe Pose | G0 | Human body → skeletal point cloud | GPU |
| all-MiniLM-L6-v2 | C | Observation text embeddings | CPU |
| LFM 2.5 | D | Dorothy reasoning, 125K context, on-device | CPU |
| E(3)-equivariant | I1 | Body-field perturbation, rotation-invariant | GPU |
| Score-Based Diffusion | I2 | Background manifold synthesis | GPU |

---

## Part XVI — Status

| Track | Status | Blocking issue |
|-------|--------|----------------|
| 0-A FieldParticle | 🔴 Not started | Unblocks everything |
| 0-B tokens.slint | 🔴 Not started | Unblocks E, F, 0-D |
| 0-C SAM gate | 🔴 Not started | Unblocks G, H |
| 0-D Hardware applet | 🔴 Not started | Needs 0-B |
| G0-1 Video point cloud | 🔴 Not started | Needs 0-A, 0-C |
| G0-2 Pose estimation | 🔴 Not started | Needs G0-1 |
| G0-3 Scene coordinate | 🔴 Not started | Needs G0-2 |
| A1 Dispatch loop | 🟡 Partial — audio hack | Needs 0-A |
| A2 RF ingester | 🔴 Not started | Needs A1 |
| A3 Video ingester | 🔴 Not started | Needs G0-1 |
| A-ENV Environmental | 🔴 Not started | Needs 0-A |
| A-HET Heterodyning | 🔴 Not started | Needs A1 |
| A-EC Phase coherence | 🔴 Not started | Needs A1 |
| A-WOFDM | 🔴 Not started | Needs A1, Pluto+ TX |
| A3-EDGE Coral/Pico/Pluto | 🔴 Not started | Needs A1 stable |
| B1–B5 TimeGNN + LNN | 🟡 Partial stubs | Needs 0-A, real corpus |
| C1–C3 Glinda | 🔴 Not started | Needs 0-A |
| D1–D3 Dorothy | 🔴 Not started | Needs C1 |
| E1–E3 Toto | 🟡 Design proven | Needs 0-B |
| F1–F4 Chronos | 🔴 Not started | Needs E1, 0-B |
| G1–G5 + sub-tracks | 🔴 Not started | Needs A1, G0-1, 0-C |
| H series | 🔴 Not started | Needs G5 |
| HA1–HA3 Haptics | 🔴 Not started | Needs G-SPH2, VCA hardware |
| I Biometric cloak | 🔴 Conceptual | Needs A–HA + NixOS |
| J1–J7 | 🔴 Post-Track-I | Full pipeline proven |
| K1–K3 rpitx | 🔴 Post-Track-H | Legal gate |

---

*This document is the single source of truth for Project Synesthesia.*
*The prior documents — ROADMAP.md, both addenda, and SYNESTHESIA_MASTERPLAN.md — are
superseded and can be deleted.*
*Update the status table when milestones complete.*
*Do not add tracks without updating the dependency graph in Part XI.*
*Do not contradict the invariant rules in Part II.*
*Every agent prompt references this document. No agent prompt supersedes it.*
