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
