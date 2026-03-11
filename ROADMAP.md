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

**Addendums Incorporated**:
- ROADMAP_ADDENDUM_physics_haptics.md: SPH particle physics, RF-BSDF materials, 600Hz haptics, EMERALD CITY phase coherence
- ROADMAP_ADDENDUM_modulation.md: W-OFDM wavelet synthesis, advanced modulation schemes, Dirty Paper Coding

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
