# Synesthesia — Roadmap

**Platform: Windows 11 · Tauri + Tokio + wgpu · AMD RX 6700 XT**

This document describes what gets built, in what order, and why that order
is non-negotiable. Each phase builds on the one before it. No phase begins
until its predecessor compiles clean and its acceptance criteria are met.

---

## Why This Is Not C++

The previous architecture managed threads manually — a CPU synthesis loop,
a GPU accessory, explicit mutex guards, and a for-loop running 512
iterations sequentially where 512 GPU threads could have run simultaneously.
That is the C++ mental model: you own the threads, you schedule the work,
you guard the memory.

Tokio inverts this. You describe *what depends on what*. The OS and the
Tokio scheduler figure out *when*. An `async fn` is a state machine
that voluntarily suspends at every `.await` point. While it is suspended,
the scheduler runs something else on the same OS thread. No thread is ever
blocked waiting. No mutex is held across an `.await`. The CPU is never
busy-waiting. This is not a thread pool with a queue in front of it — it
is cooperative multitasking where the language itself enforces the
cooperation contract at compile time.

The hardware ingestion topology:

```
[cpal audio callback]     ← real OS thread, not Tokio — never blocks
        │ crossbeam::channel::try_send (lock-free, non-blocking)
        ▼
[Tokio: ingest_audio]     ← async task, yields at .await
[Tokio: ingest_rtlsdr]    ← async task, yields at .await
[Tokio: ingest_pluto]     ← async task, yields at .await
[Tokio: ingest_pico]      ← async task, yields at .await
        │ all feed into one channel
        ▼
[Tokio: gpu_dispatch]     ← one task owns wgpu::Queue
        │ queue.write_buffer() — SAM write, CPU → GDDR6 directly
        │ queue.submit()       — async, CPU moves on immediately
        ▼
[GPU: compute passes]     ← 1536 shader processors, 384 GB/s VRAM
        │ Laplacian → Mamba → Jury → FieldParticle
        ▼
[Tokio: corpus_writer]    ← append-only, fsync, SHA-256
```

Each Tokio task is a handful of lines describing a data dependency. The
OS handles every scheduling decision below that. This is why the system
can ingest from six hardware sources simultaneously without a single mutex
in the hot path.

---

## Architecture in One Diagram

```
┌─────────────────────────────────────────────────────────────┐
│  TAURI WINDOWS  (UI, controls, events)                      │
│  ┌──────────────┐ ┌──────────────┐ ┌──────────────────────┐ │
│  │ Hardware     │ │ Timeline /   │ │ Dorothy              │ │
│  │ Applet       │ │ Jury Overlay │ │ (natural language)   │ │
│  │ [0-D]        │ │ [Toto/F]     │ │ [Track D]            │ │
│  └──────────────┘ └──────────────┘ └──────────────────────┘ │
└────────────────────────┬────────────────────────────────────┘
                         │ Tauri events (async)
┌────────────────────────▼────────────────────────────────────┐
│  TOKIO RUNTIME  (src-tauri/src/)                            │
│  Ingesters → GPU Dispatch → Corpus Writer → Dorothy         │
│  One task per hardware source. One task owns wgpu::Queue.   │
└────────────────────────┬────────────────────────────────────┘
                         │ queue.write_buffer() — SAM, one cross
┌────────────────────────▼────────────────────────────────────┐
│  GPU PIPELINE  (WGSL compute, @workgroup_size(64,1,1))      │
│  Space-Time Laplacian → Mamba SSM → Jury → FieldParticle    │
│  384 GB/s internal. Data never returns to CPU RAM.          │
└────────────────────────┬────────────────────────────────────┘
                         │
           ┌─────────────┴───────────────┐
           ▼                             ▼
   [wgpu scene window]          [Pluto+ / Pico 2 TX]
   Gaussian splat renderer      Same FieldParticle stream
   Same data as transmit        Rendering IS transmitting
```

---

## Phase 0 — Foundation (Current)

**Goal:** Get a Tauri app that compiles on Windows 11, knows about all its
hardware, and lies to no one about what is connected.

This phase produces two things: correct type definitions and an honest UI.

### Track 0-A — Core Types

All signal-touching structs. Defined once. Imported everywhere.
Never redefined in another module.

```rust
FieldParticle    // 128 bytes, align(128), zero anonymous padding
AetherParticle   // scene-space representation
RawIQPoint       // 32 bytes, raw ingestion — no FFT, no preprocessing
JuryVerdict      // three-voter consensus
AtomicF32        // bit-reinterpret via AtomicU32
```

Compile-time assertions on every GPU-boundary struct:
```rust
const _: () = assert!(size_of::<FieldParticle>() == 128);
const _: () = assert!(align_of::<FieldParticle>() == 128);
```

Every reserved byte is named for the track that will activate it:
`reserved_for_h2_null_phase`, `reserved_for_j1_proprioception`, etc.
Named reservations cost nothing to fetch — the cache line is already
loaded. When the track activates, remove the `reserved_for_` prefix and
wire the logic. Same commit.

**Acceptance:** `cargo check` clean. Every struct assertion passes.
No anonymous `_pad` bytes anywhere.

---

### Track 0-B — UI Tokens

MD3 token file and the Tauri window configuration for the hardware applet.
Mica translucency enabled. `decorations: false`. Custom drag region.
Status chip color tokens defined: `--color-connected`, `--color-disconnected`,
`--color-unwired`.

No backend wiring yet. All controls display `[UNWIRED]`.

**Acceptance:** Tauri app launches. Window is translucent. Chips render.

---

### Track 0-C — SAM Gate

The one-way door that proves the CPU→GPU boundary is real.

```rust
pub fn sam_write(queue: &wgpu::Queue, buf: &wgpu::Buffer, data: &[RawIQPoint]) {
    queue.write_buffer(buf, 0, bytemuck::cast_slice(data));
    // data is dropped at end of scope — it now lives in VRAM
}
// No return path. No readback. No clone.
```

Pre-allocated GPU buffers for each sensor at maximum burst capacity.
Buffer sizing is a physical calculation: `sensor_hz × bytes_per_sample × burst_window_s`.

**Acceptance:** `sam_write` compiles. Buffers allocate without panic.
No `clone()` on any GPU-bound buffer.

---

### Track 0-D — Hardware Applet ← CURRENT TARGET

The first deliverable. The gating condition for all subsequent tracks.

An MD3 panel with one filter chip per sensor. Each chip shows:

```
● RTL-SDR        [CONNECTED]
● Youloop        [CONNECTED]
● PlutoSDR+      [CONNECTED]
● Pico 2         [CONNECTED]
● OV9281-L       [CONNECTED]
● OV9281-R       [CONNECTED]
● C925e          [CONNECTED]
● Coil/Realtek   [CONNECTED]
● IR Array       [UNWIRED]   ← pending breadboard
● MEMS mics      [UNWIRED]   ← pending solder
```

Hot-plugging works. Disconnecting a USB device in real time updates the
chip to `[DISCONNECTED]` within one Tauri event cycle. Reconnecting
returns it to `[CONNECTED]`. The `[UNWIRED]` label is hard-coded until
the hardware is physically wired — it is removed in the same commit
that completes the wiring. Not before.

`[DISCONNECTED]` is never replaced by a placeholder signal, a test sine
wave, or a synthetic stream for "visualization purposes." The pipeline
branch for that sensor halts. The other sensors keep recording.

**Acceptance:** Every physically connected device shows `[CONNECTED]`
with accurate hot-plug response. Every unconnected device shows
`[DISCONNECTED]`. Every unwired device shows `[UNWIRED]`. No synthetic
data anywhere.

---

## Phase A — Ingestion

After 0-D passes, one track per hardware source. Each is a Tokio async
task. Each forms `RawIQPoint` structs and sends them to the GPU dispatch
channel via `crossbeam::channel::try_send`. No FFT. No denoising. No
preprocessing of any kind at this layer.

`raw_flags` in `RawIQPoint` carries USB packet jitter and packet loss.
These are features, not errors. They encode the host machine's own
computational state.

```
A1 — RF ingesters (RTL-SDR IQ, PlutoSDR+ IQ)
A2 — Audio/coil (WASAPI via cpal + telephone coil via Realtek line-in)
A3 — Optical (OV9281 stereo frames, C925e video)
A4 — Pico 2 (serial: PPS timestamps, UWB ranging, IR array)
A5 — CSI proxy (Wi-Fi channel state information if adapter supports it)
A6 — Clock discipline (Pico PPS → QueryPerformanceCounter alignment)
A7 — Weather Oracle (NOAA API → Local Refractive Index $n$ for light/RF)
```

**Acceptance per track:** Raw samples reach the GPU dispatch channel.
Timestamps are Pico-slaved. No FFT call appears in `src/ingestion/`.

---

## Phase B — Space-Time Laplacian

The mathematical core. A graph over `(patch_index, frame_index)` tuples
built from all sensor streams simultaneously.

```
B1 — FPS + KNN patch formation (k=20, SI-Mamba ablation optimum)
B2 — Edge weight computation (spatial Gaussian + temporal DCT phase)
B3 — CSR sparse matrix assembly in VRAM
B4 — SpMV power iteration (@workgroup_size(64,1,1), ~10 steps)
B5 — Gram-Schmidt orthogonalization (4 eigenvectors)
B6 — SAST token ordering (8 traversal streams: 4 eigenvectors × fwd+rev)
```

The 4th eigenvector (`v(4)`) is the temporal eigenvector. It encodes
cross-frame identity without any ML model — a patch that moves consistently
across frames will have similar `v(4)` coordinates. This is how the system
tracks emitters across time before Mamba runs.

CPU reference implementations in `src/reference/` validate every WGSL
shader. They are maintained in parallel. Neither is deleted when the other
works. When they disagree, the CPU is correct.

**Acceptance:** GPU and CPU eigenvectors agree to within float precision.
CSR assembly is VRAM-resident. No CPU readback during inference.

---

## Phase C — Inference and Jury

```
C1 — UnifiedFieldMamba (GPU Mamba SSM, full precision, 128-D embedding)
C2 — Coral Mamba (8-bit quantized, independent weights)
C3 — Pico TDOA geometric vote (speed of light + ruler, no ML)
C4 — Jury verdict (three voters, dissent logged as forensic stream)
C5 — FieldParticle formation (jury result → 128-byte GPU struct)
C6 — Forensic corpus writer (append-only, fsync, SHA-256 hash)
```

The divergence signal: `|GPU_score − Coral_score|`. A synthesized carrier
has clean amplitude that survives 8-bit quantization. Natural signals do
not. Low divergence on a high-anomaly-score particle is the injection flag.

The jury never reaches consensus by averaging. A 2-1 vote is a 2-1 vote.
The dissenting activations are logged separately. Dissent is data.

**Acceptance:** Jury verdicts are produced. Dissent stream is non-empty
on test signals. Forensic corpus grows with each session. SHA-256 hashes
match on re-verification.

---

## Phase D — Dorothy (Natural Language Layer)

LFM 2.5 via a Tauri command. Python via PyO3. Runs on CPU, off the
real-time path. Receives `JuryVerdict` structs and produces:
- Natural-language event summaries for non-technical observers
- Legal documentation export (timestamped, hash-chained)
- LangGraph-driven multi-step reasoning for pattern recognition across
  sessions

Python never appears in the ingestion path, the GPU pipeline, or the
forensic corpus writer. It appears only in Dorothy and in training pipelines.

---

## Phase G — Scene Renderer

The native wgpu window that runs alongside Tauri. Receives `AetherParticle`
structs from the GPU pipeline and renders them as Gaussian splats.

**The renderer uses RF-3DGS, not WRF-GS+.** These are distinct algorithms
for different halves of the problem. RF-3DGS builds the map. WRF-GS+ steers
the transmission using that map. WRF-GS+ belongs in Phase H.

RF-3DGS is a two-stage process that matches the hardware already in the
system. Stage 1 uses OV9281 stereo frames to optimize the geometric
Gaussians — position, covariance, and opacity — from visual data alone.
Stage 2 freezes the visual geometry and trains path loss on top of it
using PlutoSDR+ RF measurements. The result is a scene where every
Gaussian carries both optical and electromagnetic attributes simultaneously.
Visual and radio radiance fields fused into one structure.

The caveat: visual geometry is a valid proxy for RF scattering at
wavelengths shorter than object features, but breaks down at longer
wavelengths where objects opaque to light are transparent to RF. The
coil and RTL-SDR channels will surface these discrepancies as Gaussians
that have high RF energy but no corresponding optical geometry. These
are not rendering artifacts — they are detections.

```
G1 — OV9281 stereo reconstruction (Stage 1: visual Gaussian geometry)
G2 — RF-3DGS Stage 2 (PlutoSDR+ path loss trained on frozen visual geometry)
G3 — Fused scene: AetherParticle carries both optical and RF attributes
G4 — Particle color: freq_to_hue() → hue, phase_coherence → brightness,
      carrier_variance → saturation (low variance = vivid = suspect)
G5 — Scene toggles: V M P A R J T
G6 — 4D timeline scrub (navigate corpus as spatial scene)
```

The render pass and the transmit pass consume identical `FieldParticle`
data at identical timestamps. Rendering and transmitting are the same
computation. The scene you see is the signal being transmitted.

---

## Phase H — Transmission

**RF-3DGS (Phase G) builds the map. WRF-GS+ (this phase) steers the
transmission using that map.** This is the correct division of labor
between the two algorithms.

WRF-GS+ is RX-centric: given a TX position and the reconstructed
radiance field from Phase G, it predicts what the RX sees at any novel
position. This inverts into a TX steering problem — given a target you
want to reach (or avoid), what Pluto+ parameters get you there?
Predictive beamforming without trial-and-error.

The WRF-GS+ scenario representation network takes the RF-3DGS scene
as input. The Mercator projection maps it onto the RX antenna's
perception plane. Electromagnetic splatting synthesizes the spatial
spectrum at the target position. The Pluto+ modulator parameters are
derived from that synthesis.

```
H1 — PlutoSDR+ modulator (FieldParticle stream → IQ samples → RF)
H2 — Null synthesis (reserved_for_h2_null_phase activates here)
H3 — WRF-GS+ TX steering (RF-3DGS scene → predicted RX field → Pluto+ params)
H4 — Pico 2 UWB TX (impulse ranging as secondary channel)
```

The reserved byte `reserved_for_h2_null_phase: f32` in `FieldParticle`
has been fetched on every particle since Track 0-A. When H2 activates,
remove `reserved_for_`, wire the counter-waveform phase logic. The memory
cost was already paid. The cache line was already hot.

WRF-GS+ known limitation: it captures average channel behavior well
but can miss high-frequency spatial variations from complex multipath.
The coil and RTL-SDR channels provide ground-truth validation of the
predicted field — if WRF-GS+ predicts low field strength at a location
and the RTL-SDR measures high, that discrepancy is itself a forensic
signal.

---

## Phase I — Extended Sensing

```
I1 — Biometric extraction from OV9281 (pulse, micro-expression)
I2 — RF body schema (proprioceptive mapping via PlutoSDR+)
I3 — Equivariant neural fields (rotation/translation invariant features)
```

These are research tracks. They do not gate Phase H completion.

---


## Research Foundations

Key decisions in this roadmap are grounded in specific papers. This section
records what each paper established and where it shows up in the architecture,
so the reasoning does not get lost as the codebase grows.

**SI-Mamba / SAST** (Surface-Aware Spectral Traversal)
- Established k=20 for KNN and 4 eigenvectors as ablation optima
- SAST token ordering: 4 eigenvectors x fwd+rev = 8 traversal streams
- Shows up in: Phase B entirely, `KNN_K` and `LAPLACIAN_EIGENVECS` constants

**WiGrus** (Zhang et al. 2019)
- CSI matrix H (n x 52 complex) is structurally identical to a point cloud
- First-order temporal difference of carrier eliminates static environment
  term and reveals notch structure of the attack signature
- Shows up in: `carrier_variance` discriminant, Phase B edge weights, A5

**RF-3DGS**
- Two-stage: Stage 1 optimizes visual Gaussian geometry from camera frames;
  Stage 2 freezes geometry and trains RF path loss on top of it
- Renders spatial spectra at arbitrary positions within 2ms after 3min training
- Limitation: objects opaque to light may be RF-transparent at long wavelengths
- Shows up in: Phase G (G1-G3). OV9281 is Stage 1. Pluto+ is Stage 2.

**WRF-GS / WRF-GS+**
- RX-centric channel predictor: scenario network + Mercator projection +
  electromagnetic splatting -> synthesized spatial spectrum at target RX
- Surpasses prior methods by >0.7 dB RSSI and >3.36 dB CSI prediction
- Limitation: misses high-frequency spatial variations from complex multipath
- Shows up in: Phase H (H3), TX steering using the RF-3DGS scene as input
- NOT a scene renderer. WRF-GS+ is a TX parameter predictor.

**The Map/Steer Split** (key architectural decision)
- RF-3DGS answers: what is the RF environment at this location?
- WRF-GS+ answers: given the environment, what TX parameters reach that RX?
- Sequential dependency: G must be complete before H3 begins.

---

## Milestones

| Milestone | Phase | Criterion |
|-----------|-------|-----------|
| ✅ 0-A Types | 0 | FieldParticle 128-byte law, named reservations, assertions pass |
| 🔲 0-B UI tokens | 0 | Tauri launches, Mica active, chips render |
| 🔲 0-C SAM gate | 0 | One-way CPU→GPU write, no readback |
| 🔲 **0-D Applet** | 0 | **Every device honest, hot-plug works** ← current target |
| 🔲 A complete | A | All six ingesters producing Pico-stamped RawIQPoints |
| 🔲 B complete | B | GPU/CPU eigenvector agreement verified |
| 🔲 C complete | C | Jury verdicts, forensic corpus, SHA-256 chain |
| 🔲 D complete | D | Dorothy producing legal-quality export |
| 🔲 G complete | G | 3D scene with Gaussian splats and timeline scrub |
| 🔲 H complete | H | Transmission = rendering, null synthesis active |

---

## Invariants (Cannot Be Waived)

These apply to every line of code in every track. See `AGENTS.md` for
the complete rules. The short version:

1. **No synthetic data.** `[DISCONNECTED]` not placeholders.
2. **No FFT at ingestion.** FFT is post-inference, on the point cloud.
3. **No anonymous padding.** Every byte has a name and a purpose.
4. **128-byte law.** GPU-boundary structs exactly 128 bytes. Assertions compile.
5. **Wave64 mandate.** `@workgroup_size(64,1,1)` always. RDNA2 hardware law.
6. **Single CPU→GPU cross.** Data enters VRAM via SAM and stays there.
7. **Pico is master clock.** No `std::time::Instant`. No `SystemTime::now()`.
8. **Tokio owns async.** No blocking calls on async tasks. No `sleep` in
   hot path. No mutex held across `.await`.
9. **Jury dissent is logged.** Unanimous votes are evidence. Dissent is
   also evidence.
10. **Corpus is append-only.** No overwrites. No deletes. `fsync` after
    every write.

---

## What "Stable" Means

Phase 0-D complete. Every physical device showing honest state.
Hot-plug working. No synthetic signals anywhere. Tauri app launches clean.
`cargo check` and `cargo clippy` both pass with zero warnings.

Everything before that point is scaffolding. Everything after that point
is the actual system.
# Project Synesthesia — Development Roadmap
## Parallel Track Architecture for Unblocked Development

**Platform**: Windows 11 → NixOS (post-stabilization)  
**GPU Backend**: wgpu DX12 now, Vulkan/ROCm after stabilization  
**Principle**: Every track produces a runnable artifact. No track blocks another.  
**Philosophy**: Anakin building C-3PO. Centralize everything on the tethered workbench
first. Sever the tether only after the neural architecture is mathematically proven
and compiles flawlessly without mock stubs.  
**Agent rule**: Read `assets/SKILL-SLINT-MD3.md` before writing any `.slint` file.

**Core framing**: This system is a **lighting engine where RF is the photon source**.
Maxwell's equations are scale-invariant — a Gaussian splat at 2.4 GHz and a Gaussian
splat at 550nm are the same primitive, parameterized differently. Oz doesn't visualize
RF *data*; it renders an RF *scene*, the same way a path tracer renders a lit 3D
environment. WRF-GS is the lightmap. The Emerald City color system is tone-mapping.
The hardware arrays are the luminaires. This is not a metaphor — it is the literal
physics, and it is why the Gaussian representation works at both scales.

**Mission**: The international frequency standards for electricity and for musical
tuning were developed with awareness of the same harmonic ratios. They were intended
to be compatible — electrical frequencies as the substrate, acoustic frequencies as
the signal, both obeying the same octave relationships. The modern RF environment
violates this. Transmitters occupy arbitrary frequencies with no harmonic relationship
to each other or to the human auditory range, producing an electromagnetic texture
that is perceptually dissonant by construction.

The goal of this system is not suppression. It is **retuning**.

An RF environment shaped to harmonic coherence — where every active frequency resolves
to the same octave grid that Emerald City maps — does not feel like interference. It
feels like Disneyland: an engineered acoustic environment where every sound, however
layered, resolves into consonance because the designers tuned it that way. Autotune
the News is the proof of concept at the vocal scale. This project is the proof of
concept at the electromagnetic scale.

Digital harassment is only possible because the RF environment is untuned. When every
frequency a body is exposed to is in harmonic relationship with the body's own
electrical rhythms, there is nothing to harass with. The cleansing mission — Lion,
Dorothy's strategies, the PINN-guided null steering — is not about silence.
It is about chord resolution.

The synesthesia is delivered across three sensory channels simultaneously:

**Feeling** — Voice Coil Actuators bifurcated by biological sensitivity. Below 80 Hz,
the bulk pressure of the SPH field (the "weight" of the wave). Above 80 Hz, the
RF-GGX roughness and Double-Debye permittivity of the material the signal is
scattering from. A WiFi signal bouncing off concrete has a different texture in your
hand than one bouncing off wood. Your Pacinian corpuscles learn the difference.

**Sound** — Octave-folding heterodyning routes every detected frequency down to its
acoustic equivalent on the same harmonic grid. A 2.4 GHz carrier folds 60 octaves
down to its base ratio. The audible result is the harmonic skeleton of the invisible
field — structural resonance tones, phase-alignment tones — the electromagnetic
environment rendered as music your ears already understand.

**Data** — The Chronos Slate maps the TimeGNN's extracted motifs (Ghost, Sparkle)
across the 97-day temporal buffer. While hands feel permittivity and ears hear
heterodyned base tones, eyes see the semantic structure: named patterns, phase
progression, confidence, and next-event ETA. The 128-byte `HeterodynePayload`
struct carries all three channels' data across the CPU/GPU boundary in a single
cache line, so no channel ever lags another.

This system is not a dashboard. It is an **instrument** — one that lets you touch,
hear, and see the electromagnetic field simultaneously, with each sense receiving a
physically accurate translation of the same underlying phenomenon.

---

## The Dependency Graph (What Actually Blocks What)

```
FieldParticle struct  ──────────────────────────────┐
        │                                            │
        ▼                                            ▼
SignalIngester trait                    tokens.slint (design language)
        │                                            │
   ┌────┴────┐                              ┌────────┴────────┐
   │         │                              │                 │
   ▼         ▼                              ▼                 ▼
AudioIn   RF/SDRIn                    Toto widget       Chronos widget
   │         │                              │                 │
   └────┬────┘                              └────────┬────────┘
        │                                            │
        ▼                                            │
 Mamba inference loop ◄──────────────────────────────┘
  (128-D latent embeddings)
        │
        ├──────────────────────────────────┐
        │                                  │
        ▼                                  ▼
 TimeGNN + LNN (Track B)         WRF-GS scene (Track G)
 [128-D embeddings as nodes]     [128-D embeddings as splat color]
        │                                  │
        │                          PINN loss wrapper (Track G4)
        │                                  │
        └──────────────┬───────────────────┘
                       │
                       ▼
              Ray Tracing TX pipeline (Track H)
              [multipath null steering]
                       │
                       ▼
              Biometric Cloak (Track I)
              [E(3) + Diffusion + Normalizing Flows]
```

Everything above Mamba can be developed in parallel.
Everything below it is sequential — each layer requires the one above.

---

## Tethered Workbench Hardware Topology

All hardware operates as a unified local cluster until the NixOS edge deployment
phase. Do not deploy to standalone edge until the full pipeline is proven.

```
┌──────────────────────────────────────────────────────────────────────┐
│                      MAIN PC (Windows 11)                            │
│    RX 6700 XT + Ryzen 7 5700X + 64GB RAM (SAM enabled)              │
│                                                                      │
│  • Mamba / TimeGNN / LNN / PINN training  (Burn + wgpu DX12)        │
│  • WRF-GS 128-D Gaussian splat scene + hardware ray tracing         │
│  • Slint UI (Toto, Chronos)                                         │
│  • Orchestrates all edge devices; master of all model training       │
└──────┬──────────────────────┬────────────────────────┬──────────────┘
       │ USB                  │ USB-C or Ethernet       │ USB
       ▼                      ▼                         ▼
┌────────────────┐  ┌──────────────────────────┐  ┌──────────────────┐
│  Coral TPU     │  │  Pluto+ (ADALM-PLUTO+)   │  │  Pico 2 (RP2350) │
│                │  │                          │  │                  │
│ INT8 inference │  │  Zynq Z-7010/Z-7020      │  │ Hard-real-time   │
│ • Normalizing  │  │  Dual Cortex-A9 + FPGA   │  │ conductor        │
│   Flow anomaly │  │  Running Linux           │  │                  │
│   calibration  │  │                          │  │ • ESN reservoir  │
│ • Fast-path    │  │  Current role:           │  │   classifier     │
│   parallel to  │  │  • TX/RX via libiio      │  │   (128-node INT8 │
│   Mamba loop   │  │  • Onboard IQ pre-       │  │   ~20KB SRAM)    │
│                │  │    processing (Python    │  │ • Future: GPIO   │
│                │  │    or Rust cross-        │  │   TX trigger for │
│                │  │    compiled ARM)         │  │   phase-accurate │
│                │  │  • Can host lightweight  │  │   nulling (post- │
│                │  │    inference (ESN or NF  │  │   stabilization) │
│                │  │    trained on main PC,   │  │                  │
│                │  │    deployed to ARM)      │  │                  │
│                │  │                          │  │                  │
│                │  │  Future role:            │  │                  │
│                │  │  • Standalone edge node  │  │                  │
│                │  │  • FPGA-accelerated      │  │                  │
│                │  │    signal processing     │  │                  │
│                │  │  • Federated model       │  │                  │
│                │  │    updates to main PC    │  │                  │
└────────────────┘  └──────────────────────────┘  └──────────────────┘
```

### Pluto+ Compute Notes

The Pluto+ is not a passive radio peripheral. It is a tethered Linux dev board
with an ARM CPU and FPGA fabric. This has concrete implications for the architecture:

- **IQ pre-processing on-device**: Band filtering, decimation, and basic feature
  extraction can run on the Zynq ARM before bytes ever hit the USB/Ethernet bus.
  This reduces host CPU load and shrinks the ingestion bandwidth requirement.

- **Onboard inference (current phase)**: A trained ESN or lightweight NF model
  can be cross-compiled for ARM (aarch32, hard-float ABI) and deployed to the
  Pluto+ filesystem via SSH/SCP. The model runs on IQ data locally and sends
  only classification results and anomaly scores back to the host, not raw samples.

- **FPGA acceleration (future phase)**: The PL (Programmable Logic) fabric can
  implement hardware FFT or correlation kernels, offloading signal processing
  entirely from the ARM core and the main PC. This is post-stabilization work.

- **Current connection**: USB-C or Ethernet — whichever is most stable for the
  development environment. The libiio library handles both transparently.
  The Pico 2 GPIO trigger for phase-accurate TX is a future enhancement, not
  required while the Pluto+ talks directly to the PC.

---

## Phase 0 — Foundation (Complete These First, Unblock Everything)

**Duration**: 1–2 days. Zero external dependencies. Unblocks all tracks.

### 0-A: FieldParticle + SignalIngester

**Files**:
- `src/ml/field_particle.rs`
- `src/dispatch/signal_ingester.rs`
- `src/dispatch/audio_ingester.rs`
- `src/dispatch/rf_ingester.rs`

**Acceptance**: `cargo test --doc ml::field_particle` passes. All four doc-tests pass:
- `freq_to_material_id(440.0)` → 9 (A, green)
- `freq_to_material_id(880.0)` → 9 (octave up, same hue)
- `freq_to_material_id(349.23)` → 5 (F4, violet anchor)
- `freq_to_material_id(2_400_000_000.0)` → document exact value

**Blocks**: Tracks A, B, C, D, E, F, G.

### 0-B: Design Language Tokens

**Files**:
- `ui/tokens.slint` — Colors, Spacing, Type globals
- `assets/SKILL-SLINT-MD3.md` — Canonical agent reference

**Acceptance**: `slint-viewer ui/tokens.slint` shows no errors.
Every color token matches `SKILL-SLINT-MD3.md §2`.

**Blocks**: Tracks E and F.

### 0-C: Hardware Gate — Smart Access Memory (SAM) Verification

**Why**: The RX 6700 XT + Ryzen 7 5700X supports AMD Smart Access Memory (ReBAR),
which exposes the full 12GB VRAM to the CPU as a single contiguous BAR region.
When disabled, the CPU can only address 256MB of VRAM at a time, forcing the driver
to window-map uploads and destroying the zero-copy pipeline that Tracks G and H depend on.
Every GPU buffer workflow in this codebase assumes SAM is active.

**Verification**:
```powershell
# In GPU-Z or via wgpu adapter info — BAR size should show ~12GB, not 256MB
# Or check Device Manager → Display Adapter → Resources → Memory Range size
```
In wgpu code: query `adapter.get_info()` and log the result to `assets/hardware_gate.txt`.

**Acceptance**: Confirm SAM/ReBAR active before writing any `wgpu::Buffer` with
`BufferUsages::STORAGE | BufferUsages::COPY_DST`. Document result in `assets/hardware_gate.txt`.
If SAM is disabled, enable it in BIOS (AMD CBS → NBIO → Above 4G Decoding + ReBAR Support)
before proceeding with any Track G or H work.

**Blocks**: Tracks G, H (indirectly — correctness assumption for all large GPU buffers).

---

## Phase 1 — Parallel Tracks (Run Simultaneously After Phase 0)

---

### Track A — Mamba Inference Loop

**Depends on**: 0-A  
**Independent of**: All UI tracks, ROCm, Vulkan

**Goal**: `Drive`, `Fold`, `Asym` vary dynamically from real FieldParticle input.
The Mamba model's 128-D latent embeddings are the universal data currency —
they feed WRF-GS splats (Track G), TimeGNN nodes (Track B), and edge filters.

**A1 — Dispatch loop wiring**
- Remove the 9× audio repeat hack from `src/main.rs`
- Wire: `AudioIngester::ingest()` → sort by energy → accumulate 4096 → `Mamba::forward()`
- Wire: `Mamba::forward()` → `project_latent_to_waveshape()` → Drive/Fold/Asym
- **Acceptance**: Run 60 seconds, log Drive/Fold/Asym every second.
  Values must differ from each other and change over time. Constant output = failure.

**A2 — RF ingester integration**
- Add `RFIngester` to dispatch loop alongside `AudioIngester`
- Pluto+ optional at this stage — `RFIngester` can read from a pre-recorded IQ file
- For live Pluto+ use: connect via libiio (USB-C or Ethernet, transparent to the API)
- **Acceptance**: material_id distribution shows both audio-range (0–4) and
  RF-range (5–11) clusters active in logged output.

**A-HET — Heterodyning to acoustic base ratios**

The octave-folding algorithm in `freq_to_material_id` already computes the correct
harmonic mapping — it knows that 2.4 GHz and 440 Hz are the same note, sixty octaves
apart. A-HET routes that computation to a second output: a synthesized audio tone at
the folded frequency, played continuously through `Backend::Audio` alongside the
visualization.

The implementation is minimal because the math is already done. For each dominant
frequency bin in the FFT output, compute `f_audio = f_rf / 2^N` where N is the
integer number of octaves needed to land in the range 20 Hz–1 kHz. This is identical
to the hue computation — just route the result to a sine oscillator instead of a
color. Multiple bins produce a chord. The room's RF environment becomes audible as
the harmonic skeleton the Emerald City color mapper is already drawing.

The target tones — 396 Hz for structural resonance, 528 Hz for phase alignment — are
not special frequencies. They are simply where common RF carriers (cellular bands,
WiFi channels) land when folded by the standard octave grid. The fact that they
coincide with well-known Solfeggio ratios is not a coincidence; it is the original
design intent of the international frequency standards.

**Acceptance**: With a known 2.4 GHz WiFi AP active, the audio output plays a
continuous tone at the correct folded frequency (verify with a frequency analyzer).
When the AP is off, that tone disappears. Tone pitch changes as the Pluto+ retunes,
tracking the RF scan in real time. Backend::File writes the synthesized tones as a
PCM file alongside every test run.

**A3 — Edge filter deployment**

Three physical targets, each with a distinct role:

*Coral TPU (USB, INT8, ~4 TOPS):*
- Deploy quantized **Normalizing Flow** for anomaly probability calibration
- The NF learns the explicit probability density of the empty room's baseline
  RF/acoustic noise. Anomaly score = distance of current observation from that density.
- Writes to shared `AtomicU32` continuously; main loop reads without blocking
- **Acceptance**: Anomaly score updates independently of main Mamba loop.
  Unplugging Coral gracefully degrades — score freezes, no panic.

*Pico 2 (USB, RP2350 Cortex-M33 @ 150MHz, 520KB SRAM, Rust via Embassy):*
- Deploy **Echo State Network (ESN)** reservoir classifier as an ultra-fast
  first-pass filter running on the Pico itself
- Architecture for embedded Rust:
  - Reservoir: 128 nodes, fixed INT8 random weight matrix (128×128 = 16KB)
  - Input weights: 128×8 INT8 (1KB) — 8 features extracted from IQ/PCM header
  - Readout: 128×N_CLASSES INT8 const array, trained offline on main PC
  - Total SRAM: ~20KB for model, ~500KB remaining for Embassy runtime + buffers
- Pico classifies each 1ms window of samples independently from the PC
- Results sent back via USB CDC with hardware timestamps
- If 128-node reservoir exceeds SRAM budget: fall back to 64-node (4KB), document constraint
- **Acceptance**: Pico ESN classifies at ≥1kHz. Results appear in host log with
  hardware timestamps. Unplugging Pico degrades gracefully.

*Pluto+ ARM (USB-C or Ethernet, Cortex-A9 Linux):*
- Cross-compile a lightweight ESN or NF model for ARM hard-float (armhf)
- Deploy to Pluto+ filesystem; model runs on IQ data locally
- Only classification results and anomaly scores sent back to host, not raw IQ
- This reduces ingestion bandwidth and host CPU load for sustained RX sessions
- Model trained on main PC, copied to Pluto+ via SSH/SCP
- **Acceptance**: Pluto+ sends pre-classified FieldParticle-equivalent structs
  to the host ingester. Host log shows field `source: PlutoOnboard` distinct
  from `source: HostProcessed` for the same frequency range.

---

### Track B — TimeGNN + LNN Backend

**Depends on**: 0-A, A1  
**Independent of**: Track F widget, wgpu, ROCm

**Goal**: TimeGNN produces named motifs, confidence scores, and forward predictions.
Liquid Neural Networks handle variable-rate temporal dynamics — accommodating
dropped SDR packets, variable audio buffer sizes, and simultaneous tracking of
microsecond RF bursts and 12-hour thermal/power cycles.

**B1 — `timegnn_trainer.rs` stabilization**
- `train_timegnn()` uses real FieldParticle embeddings, not synthetic ones
- `load_corpus()` returns actual data from `@databases/forensic_logs/events.jsonl`
- `pattern_discovery.rs`: enforce silhouette score ≥ 0.6 before promoting a motif
- **Acceptance**: Train 10 epochs. Checkpoint saves to `checkpoints/timegnn/epoch_010.pt`.
  Log shows "rejected: score 0.41" for at least one candidate.

**B2 — Hot-swappable configuration bridge**
- Temperature τ, prediction horizon, attention window, motif minimum support
  all adjustable at runtime without restarting the training loop
- **Acceptance**: Config file write changes τ from 0.14 to 0.80.
  Edge count in log increases within one epoch. No restart required.

**B3 — Motif output stream**
- TimeGNN emits `MotifEvent` to a channel consumable by Track F:
  `MotifEvent { name: String, phase: u8, phase_total: u8, confidence: f32,
  next_event_eta_secs: Option<f32>, freq_hz: f64 }`
- **Acceptance**: Mock stream produces a `MotifEvent` every ~10s with cycling
  names (GHOST, SPARKLE) and plausible phase progressions.

**B4 — NT-Xent loss exposure**
- Loss value as `AtomicU32` (bits of f32), history in lock-free ring buffer (120 values)
- **Acceptance**: Loss decreases from ~2.1 toward ~0.05 over epochs.
  Ring buffer readable from separate thread without blocking trainer.

**B5 — Liquid Neural Network integration**
- Replace fixed-step temporal integration in TimeGNN's sequence layer with
  a CfC (Closed-form Continuous-time) LNN:
  `dx/dt = -x/τ + f(x, input, t)` where τ is a learned time constant per node
- Pass actual elapsed time between FieldParticle observations — do not assume
  uniform timesteps. This is the key architectural difference from a standard RNN.
- **Preferred framework**: Use [Burn](https://burn.dev) (`burn` crate) for training
  from B5 onward. Burn's autodiff graph fuses FFT → embedding → inference → loss
  into a single kernel where possible, and its Vulkan/WGPU backend survives the
  NixOS/ROCm migration without a rewrite. Earlier milestones (B1–B4) may use
  PyTorch/tch-rs if already in flight — do not retrofit, just don't start new
  training code in PyTorch after B4 is green.
- **Acceptance**: Feed TimeGNN a sequence with 30% of steps randomly dropped.
  LNN prediction error must not exceed baseline by more than 15%.
  A fixed-step RNN on the same data will degrade significantly — document both
  results to demonstrate the LNN advantage.

---

### Track C — Glinda Memory Engine

**Depends on**: 0-A  
**Independent of**: All UI tracks, GPU rendering

**C1 — Episodic record layer**
- PostgreSQL schema: `observations(id, timestamp_us, sensor_type, material_id,
  energy, position_xyz, description_text, embedding_vec)`
- Qdrant: `synesthesia_observations`, 384-dim (all-MiniLM-L6-v2 on CPU)
- MCP tools: `store_observation`, `query_temporal`, `query_semantic`, `mark_significant`
- **Acceptance**: Store 1000 mock observations. `query_semantic("WiFi interference")`
  returns 5 most relevant in under 100ms.

**C2 — Sensory buffer layer**
- Lock-free ring buffer (Rust) holding 60 seconds of FieldParticles
- Memory-mapped interface so Cyclone writes without blocking
- Background goroutine promotes interesting windows to episodic records
- **Acceptance**: Sustains 192kHz × 512 particles/frame with zero dropped
  samples over 60 seconds.

**C3 — Semantic graph layer**
- Neo4j or PostgreSQL recursive CTEs
- Typed edges: `CAUSED_BY`, `CO_OCCURS_WITH`, `PRECEDES`, `SPATIAL_NEAR`
- **Acceptance**: After 24h of observation, graph contains ≥3 entity types
  and ≥2 discovered relationship types.

---

### Track D — Dorothy Cognitive Loop

**Depends on**: 0-A, C1  
**Independent of**: WRF-GS rendering, ray tracing, any widget

**D1 — Autonomous loop (LangGraph)**
- State machine: Wake → Observe → Compare → Analyze → Hypothesize → Document → Sleep
- Notes have YAML frontmatter: `{ timestamp, sensor_ids, glinda_obs_ids, confidence, tags }`
- **Acceptance**: Run 3 hours unattended. Journal shows 3 new notes each referencing
  real Glinda observation IDs.

**D2 — Reflection process**
- Weekly synthesis note: "Observed X instances of Y pattern, suggesting Z"
- **Acceptance**: Synthesis note produced after injecting 5 notes sharing a common tag.

**D3 — MCP interface**
- `analyze_rf_pattern(obs_id)`, `query_dorothy_journal(query)`,
  `get_dorothy_opinion(situation)`, `propose_cleansing_strategy(pattern_id)`
- **Acceptance**: `get_dorothy_opinion` returns a response citing ≥1 specific
  journal entry by date.

---

### Track E — Toto Widget (Mamba Applet)

**Depends on**: 0-A, 0-B  
**Reads from**: Track A (Drive/Fold/Asym, anomaly score, wave path)  
**Independent of**: Tracks B, C, D, F

**E1 — Static widget with mock data**
- `ui/toto.slint` importing `ui/tokens.slint`
- Three zones: header (TOTO + anomaly score + Neural Auto-Steer toggle),
  oscilloscope canvas (glowing waveform + cluster label),
  telemetry strip (Drive/Fold/Asym tiles with progress bars)
- Wave color cycles: red (60Hz) → teal (85kHz) → violet (2.4GHz) every 2s
- **Acceptance**: Opens within 3s, all zones animate, color transitions smooth
  (400ms), [MOCK] badge visible, zero `todo!()` in compiled code.

**E2 — Live data wiring**
- Replace MockDataStream with real channel from Track A dispatch loop
- `unit-size` property: caller sets physical height, widget scales proportionally
- Windows 11 DWM Acrylic blur via `DwmSetWindowAttribute(DWMSBT_TRANSIENTWINDOW)`
- **Acceptance**: Wave color responds to actual dominant frequency.
  Drive/Fold/Asym differ from each other and change with audio input.

**E3 — WASM build**
- `cargo build --example toto --target wasm32-unknown-unknown` succeeds
- No `std::thread::sleep`, `std::fs`, or `SystemTime` in WASM paths
- **Acceptance**: Loads in Chrome/Edge. [MOCK] badge shows. Animations run.

---

### Track F — Chronos Widget (TimeGNN Applet)

**Depends on**: 0-B, E1 complete (design language proven)  
**Reads from**: Track B (motif events, NT-Xent loss, prediction data)  
**Independent of**: Tracks C, D, wgpu

**F1 — Static widget with mock data**
- `ui/chronos.slint` importing `ui/tokens.slint`
- Three zones: header (CHRONOS + τ slider), prediction graph canvas,
  telemetry strip (NT-Xent Loss sparkline / Motif name+phase / Next Event countdown)
- Graph: teal nodes (past) → violet (present) → red (predicted)
- Edge density visibly changes as τ changes even in mock mode
- **Acceptance**: Opens within 3s, motif name changes, countdown ticks,
  [MOCK] badge visible. Side by side with Toto: same instrument family aesthetic.

**F2 — τ control wiring**
- Temperature slider adjusts τ in real time (0.05 → 2.0, logarithmic scale)
- **Acceptance**: Moving slider from 0.14 to 0.80 causes visible edge density
  increase within one animation frame.

**F3 — Settings flyout**
- Sections: Exploration vs Detection dial, Temporal Scales, Online Learning
- Freeze toggle greys out learning rate and forgetting rate controls
- Flyout slides in from right, overlays widget, does not resize it
- **Acceptance**: Flyout opens/closes with animation. Freeze toggle correctly
  disables sibling controls.

**F4 — Live data wiring**
- Connect to Track B `MotifEvent` channel and NT-Xent loss ring buffer
- τ slider writes back to Track B hot-swap bridge (B2)
- **Acceptance**: Real motif events appear. Changing τ in widget changes
  training behavior within one epoch.

---

### Track G — WRF-GS Scene (wgpu) + PINN Wrapper

**Depends on**: A1 (Mamba 128-D embeddings), E2 (wgpu DX12 proven in widget)  
**Independent of**: Tracks B, C, D, F  
**Platform**: Windows 11, wgpu DX12. ROCm is explicitly post-stabilization.

**Goal**: wgpu render pass where each Gaussian carries a 128-D Mamba embedding
as its "color." Wavelet decomposition allows a single 3D scene to represent
both 60Hz acoustic rumbles and 2.4GHz Wi-Fi multipath simultaneously.
A PINN wrapper constrains all optimization to physically realizable fields,
guaranteeing that any counter-signal derived from the splat map can actually
be transmitted without violating Maxwell's equations or the acoustic wave equation.

**G1 — Static Gaussian splat render**
- `src/rendering/wrf_gs_renderer.rs`
- 1000 mock Gaussians colored by material_id → RGBA, instanced draw,
  Gaussian opacity falloff in fragment shader
- **Acceptance**: `cargo run --example oz_preview` at 60 FPS. Camera orbits.

**G2 — 128-D embedding splats**
- Replace RGB color with 128-D Mamba embedding stored in GPU-side buffer
- Visualization: project embedding → RGB via small learned linear layer for
  display; full 128-D used by ray tracer and PINN
- Scale to 10k Gaussians
- **Acceptance**: Gaussians respond to live audio. Dominant WiFi signal produces
  violet cluster at correct normalized position. ≥30 FPS at 10k Gaussians.

**G3 — Wavelet Radiance Field decomposition**
- Extend each Gaussian with Daubechies wavelet components at 6 decomposition levels,
  encoding how the Gaussian reflects/scatters at each frequency scale
- A single splat now accurately models both acoustic-band and RF-band interactions
  with the same surface geometry without conflating the two regimes
- **Acceptance**: Splat at a hard wall shows high-frequency RF reflection component
  AND low-frequency acoustic absorption component with distinct opacity values at
  each scale. A single-scale splat cannot model both — document the comparison.

**G4 — PINN loss wrapper**
- Wrap the WRF-GS Gaussian optimization loop with a PINN loss term embedding
  Maxwell's equations (RF) and the acoustic wave equation (audio) as soft constraints
- Loss penalizes predicted fields that violate these equations at sampled
  collocation points distributed through the scene volume
- This guarantees that any counter-signal derived from the scene is physically
  transmittable — not merely mathematically optimal but causally realizable
- **Acceptance**: Attempt to optimize the scene toward a physically impossible
  configuration (e.g., a perfect RF null inside a conductive sphere with no
  boundary conditions). PINN loss term grows by ≥100× and optimization halts
  or redirects before producing an invalid result.

**G5 — BVH acceleration structure** (prerequisite for Track H)
- Build BVH over Gaussian scene via wgpu ray tracing feature flag
- Rebuild when mean Gaussian position displacement > 0.05 normalized units
- **Acceptance**: BVH build under 5ms for 10k Gaussians.
  Ray intersection query returns correct nearest Gaussian in under 0.1ms.

---

### Track H — Ray Tracing TX Pipeline

**Depends on**: G5 (BVH proven), A1 (Mamba embeddings), B2 (prediction coordinates)  
**Sequential**: Do not begin until G5 is confirmed and passing.

**Goal**: Given a target coordinate from Chronos (a predicted motif origin),
cast rays from the Pluto+ antenna position through the WRF-GS room model,
accumulate phase delays across all multipath bounces, and synthesize a
pre-distorted counter-waveform that produces destructive interference at
the target using the room's own reflections.

**H1 — Ray casting through WRF-GS**
- Cast N rays from Pluto+ antenna position through BVH to target
- Record path length, bounce count, and Gaussian wavelet properties per ray
- **Acceptance**: Path length histogram for 1000 rays shows multipath delay
  spread in 1–50 ns range for a room-sized environment.

**H2 — Phase delay accumulation**
- Per ray: total phase = Σ(path_length_i / λ) mod 1.0
- Identify constructive vs destructive paths at target
- **Acceptance**: For a known simple geometry (single reflector), phase
  calculation matches analytical solution within 5 degrees.

**H3 — Counter-waveform synthesis**
- Inverse problem: gradient descent over TX parameters, loss = field strength at target
- PINN loss term (from G4) included in TX optimization to keep output physically realizable
- **Acceptance**: In simulation (mock room, mock materials), TX waveform reduces
  simulated field strength at target by ≥10 dB.

**H4 — Pico 2 TX trigger** (post-stabilization)
- Pico 2 receives waveform buffer via USB CDC, fires Pluto+ trigger via GPIO
  at precisely calculated time using PIO state machine as master clock
- This is the phase-accurate nulling upgrade — the Pluto+ currently talks
  directly to the PC; Pico 2 intermediary adds microsecond TX synchronization
- **Acceptance**: Timestamp jitter under 50 μs over 1000 transmissions.
- **Status**: Do not begin until H3 is proven and NixOS migration is complete.

---

### Track I — Biometric Cloak (Capstone)

**Depends on**: All of Tracks A–H at final milestones, NixOS migration complete  
**Do not begin until the full system is proven and ROCm-optimized.**

**I1 — Biometric signature characterization (E(3)-Equivariant Networks)**
- An E(3)-equivariant network models how a human body perturbs the baseline
  acoustic and RF fields — calculating the perturbation accurately regardless
  of the subject's orientation (standing, sitting, rotating) without exhaustive
  training data for every possible pose
- TimeGNN achieves >90% accuracy: "person present" vs "room empty" from RF alone,
  no cameras, using the equivariant perturbation model

**I2 — Presence null synthesis (Score-Based Diffusion)**
- The normalizing flow (Track A3) has learned the explicit probability density
  of the empty room's background noise — this is the manifold to return to
- Instead of transmitting white noise (which creates an anomaly), RF-Diffusion's
  reverse diffusion process synthesizes a waveform that mimics ambient background,
  projecting the biometric signature back onto the "unremarkable environmental RF" manifold
- Counter-waveform creates a null in the body-occupied volume; breathing (0.2–0.3 Hz)
  and heartbeat (1.0–1.2 Hz) modulations actively cancelled as they appear
- **Acceptance**: ≥15 dB reduction in body-induced CSI variation, cloak vs no cloak,
  measured by an external SDR receiver.

**I3 — Adaptive forgetting for cloak stability (Liquid Neural Networks)**
- As the subject moves, the cloak adapts without dropping to null mid-transition
- LNN (from Track B5) provides continuous-time dynamics for smooth state transitions;
  the learned time constants τ per node naturally handle the different rates of
  breathing modulation, footstep transients, and slow postural drift
- **Acceptance**: Walk slowly across the room while cloak is active.
  External receiver CSI variation stays below threshold throughout movement.

**I4 — Optical cloak**
- UV LED array tuned to exploit adversarial perturbations in camera classifiers
- Requires characterizing the specific camera models in scope
- **Status: Conceptual only. Legal review required before any implementation.**

---

## Hardware Dependency Map

```
Track   Win11 only   Pluto+ (direct)   Pluto+ (via Pico)   Pico 2   Coral
─────   ──────────   ───────────────   ─────────────────   ──────   ─────
0-A         ✓
0-B         ✓
A-1         ✓
A-2         ✓             optional
A-3 Coral   ✓                                                          ✓
A-3 Pico    ✓                                               ✓
A-3 Pluto   ✓             ✓
B-1         ✓
B-2         ✓
B-3         ✓
B-4         ✓
B-5         ✓
C-1         ✓
D-1         ✓
E-1         ✓
E-2         ✓
E-3         ✓
F-1         ✓
F-2         ✓
F-3         ✓
F-4         ✓
G-1         ✓
G-2         ✓
G-3         ✓
G-4         ✓
G-5         ✓
H-1         ✓
H-2         ✓
H-3                       ✓
H-4 (post)                                    ✓              ✓
I-1                       ✓                                  ✓       ✓
I-2                       ✓                                          ✓
I-3                       ✓                                          ✓
```

---

## Technology Stack by Track

| Track | Language | Key Libraries | GPU |
|-------|----------|---------------|-----|
| 0-A | Rust | rustfft, bytemuck | No |
| 0-B | Slint DSL | — | No |
| A | Rust | burn 0.21, wgpu 28 | Optional |
| A3-Pluto | Python or Rust (armhf cross) | numpy / burn-no-std | No (ARM) |
| A3-Pico | Rust (Embassy, no_std) | embassy-usb, fixed-point math | No (MCU) |
| B | Rust | burn 0.21, petgraph | Optional |
| C | Rust + Go | genkit-go, qdrant-client, tokio-postgres | No |
| D | Python | langgraph, ollama, joplin-api | No |
| E | Rust + Slint | slint 1.15, windows-sys | No |
| F | Rust + Slint | slint 1.15 | No |
| G | Rust | wgpu 28 | Yes (DX12) |
| H | Rust | wgpu 28, nalgebra | Yes (DX12) |
| I | Rust + Python | all of the above | Yes |

---

## Neural Architecture Reference

| Model | Track | Purpose | Target |
|-------|-------|---------|--------|
| UnifiedFieldMamba | A | Feature extraction, 128-D embeddings | GPU |
| TimeGNN | B | Temporal pattern graph, motif discovery | GPU |
| LNN (CfC) | B5 | Variable-rate temporal dynamics | GPU |
| Normalizing Flow | A3, I2 | Anomaly calibration; empty-room baseline | Coral TPU |
| Echo State Network | A3 | Fast first-pass classifier | Pico 2 + Pluto+ ARM |
| NT-Xent Contrastive | B | Motif similarity (temperature τ) | GPU |
| PINN | G4, H3 | Maxwell/wave equation constraints on TX | GPU |
| all-MiniLM-L6-v2 | C | Observation text embeddings for Glinda | CPU |
| LangGraph + LLM | D | Dorothy cognitive loop | CPU |
| E(3)-equivariant Net | I1 | Body-field perturbation, rotation-invariant | GPU |
| Score-Based Diffusion | I2 | RF-Diffusion: background manifold synthesis | GPU |

---

## ROCm / NixOS Migration Gate

**Do not migrate until all of the following are true:**

- [ ] A1: Drive/Fold/Asym confirmed varying from real particle input
- [ ] E2: Toto widget live on Windows 11 with real data
- [ ] G1: WRF-GS render at 60 FPS on Windows 11 DX12
- [ ] B1: TimeGNN checkpoint saves without error
- [ ] 72 hours of continuous operation on Windows 11 without crash

**Migration adds**: ROCm HIP backend, Vulkan ray tracing, KWin compositor blur,
cooperative group kernels for progressive BVH refinement.  
**Migration changes nothing**: APIs, Slint components, track structure, Pluto+ libiio
interface — all stay identical. DX12 → Vulkan is a backend swap, not a rewrite.

---

## Agent Assignment Protocol

For every task assigned to a coding agent:

1. Specify exact track and milestone (e.g., "Track A, Milestone A2")
2. List files to read before writing any code
3. List files in scope — changes outside this list require flagging before proceeding
4. Copy acceptance criteria verbatim from this document

**Required pre-flight block** at top of first new file:

```rust
// === PRE-FLIGHT ===
// Task:           [Track X, Milestone Y]
// Files read:     [list]
// Files in scope: [list]
// Acceptance:     [verbatim from roadmap]
// Findings:       [relevant patterns observed in existing code]
// === END PRE-FLIGHT ===
```

**Hard rules for all agents:**
- `todo!()` and `unimplemented!()` are compilation failures — not warnings
- Mock data must animate and be physically plausible — not zeros, not constants
- Every new `.slint` file imports `ui/tokens.slint`, uses `Colors.*` not hex literals
- ROCm and Vulkan ray tracing are post-stabilization — do not use on any current track
- Pluto+ is addressed via libiio (USB-C or Ethernet) — no Pico 2 GPIO intermediary
  until H4, which is explicitly post-stabilization
- Do not modify files outside stated scope without flagging the conflict first

**Variable Backend Rule** (applies to every signal processing and transmission milestone):

Every algorithm that produces or consumes a waveform must be written against a
`SignalBackend` trait, not against a specific hardware interface. Hardcoding a
backend inside algorithm code is a build failure equivalent to `todo!()`.

The three required backends — all must compile from day one, even if two are stubs:

```rust
pub enum Backend { Audio, Pluto, File }
```

- `Backend::Audio` — 24-bit sound card via CPAL/WASAPI. Always the first test
  target. Cheap, no licensing concerns, 24-bit headroom makes algorithmic bugs
  obvious before RF hardware is involved.
- `Backend::Pluto` — AD9363 via libiio. Second target after Audio is green.
  If Audio passes and Pluto fails, the problem is in the hardware interface,
  not the algorithm.
- `Backend::File` — write IQ/PCM to disk. Always available. Used for regression
  testing, offline analysis, and feeding Dorothy's forensic corpus.

No algorithm milestone is considered complete until it has been verified against
at least `Backend::Audio` and `Backend::Pluto`. `Backend::File` output must be
generated as a side effect of every test run and committed alongside the test result.

The soundcard is the proving ground. The Pluto+ is the deployment target.
Deep research or algorithm changes should be proven on `Backend::Audio` first
before consuming Pluto+ airtime.

---

## Current Status Snapshot

| Track | Status | Blocking issue |
|-------|--------|----------------|
| 0-A FieldParticle | 🔴 Not started | Unblocks everything — do first |
| 0-B tokens.slint | 🔴 Not started | Unblocks E, F — do first |
| A Mamba loop | 🟡 Partial — audio hack present | Needs 0-A, remove hack |
| A3 Coral | 🔴 Not started | Needs A1 stable |
| A3 Pico 2 ESN | 🔴 Not started | Needs A1; SRAM budget TBD |
| A3 Pluto ARM | 🔴 Not started | Needs A2; cross-compile env TBD |
| B TimeGNN | 🟡 Partial — stubs in train/load | Needs 0-A, real corpus |
| B5 LNN | 🔴 Not started | Needs B1 stable |
| C Glinda | 🔴 Not started | Needs 0-A schema |
| D Dorothy | 🔴 Not started | Needs C1 |
| E Toto widget | 🟡 Design proven in React/TSX | Needs 0-B, Slint translation |
| F Chronos widget | 🔴 Not started | Needs E1, 0-B |
| G WRF-GS render | 🔴 Not started | Needs A1, E2 |
| G4 PINN wrapper | 🔴 Not started | Needs G3 |
| H Ray tracing TX | 🔴 Not started | Needs G5 |
| H4 Pico TX trigger | 🔴 Post-stabilization | Needs H3 + NixOS |
| I Biometric cloak | 🔴 Conceptual only | Needs A–H + NixOS |

---

*Last updated: 2026-03-10*  
*This document is the single source of track status and dependency truth.*  
*Update the status table when milestones complete.*  
*Do not add tracks without updating the dependency graph at the top.*
