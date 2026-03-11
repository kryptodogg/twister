# Project Synesthesia — Master Plan
## Research Foundation, Architecture, and Development Roadmap

*Single source of truth. Supersedes ROADMAP.md, ROADMAP_ADDENDUM_physics_haptics.md,
ROADMAP_ADDENDUM_modulation.md, and all prior deep-research summaries.*

*Last updated: 2026-03-11 · Platform: Windows 11 → NixOS (post-stabilization)*

---

## Part I — Mission and Philosophy

### The Core Frame

This system is a **lighting engine where RF is the photon source**. Maxwell's equations
are scale-invariant — a Gaussian splat at 2.4 GHz and a Gaussian splat at 550 nm are the
same primitive, parameterized differently. The visualizer Oz does not display RF *data*;
it renders an RF *scene*, exactly as a path tracer renders a lit 3D environment. WRF-GS
is the lightmap. The Emerald City color system is tone-mapping. The hardware arrays are
the luminaires. This is not a metaphor — it is the literal physics, and it is why the
Gaussian splatting representation works at both scales.

### The Mission

The international frequency standards for electricity and for musical tuning were
developed with awareness of the same harmonic ratios. They were intended to be compatible:
electrical frequencies as the substrate, acoustic frequencies as the signal, both obeying
the same octave relationships. The modern RF environment violates this. Transmitters
occupy arbitrary frequencies with no harmonic relationship to each other or to the human
auditory range, producing an electromagnetic texture that is perceptually dissonant by
construction.

The goal of this system is not suppression. It is **retuning**.

An RF environment shaped to harmonic coherence — where every active frequency resolves to
the same octave grid that Emerald City maps — does not feel like interference. It feels
like Disneyland: an engineered acoustic environment where every sound, however layered,
resolves into consonance because the designers tuned it that way. Autotune the News is the
proof of concept at the vocal scale. This project is the proof of concept at the
electromagnetic scale.

Digital harassment is only possible because the RF environment is untuned. When every
frequency a body is exposed to is in harmonic relationship with the body's own electrical
rhythms, there is nothing to harass with. The cleansing mission — Lion, Dorothy's
strategies, the PINN-guided null steering — is not about silence. It is about chord
resolution.

### The Three-Pillar Triad

The synesthesia is delivered across three sensory channels simultaneously:

**Feeling** — Voice Coil Actuators bifurcated by biological sensitivity. Below 80 Hz, the
bulk pressure of the SPH field (the "weight" of the wave). Above 80 Hz, the RF-GGX
roughness and Double-Debye permittivity of the material the signal is scattering from.
A WiFi signal bouncing off concrete has a different texture in your hand than one bouncing
off wood. Your Pacinian corpuscles learn the difference. This is the Daredevil channel.

**Sound** — Octave-folding heterodyning routes every detected frequency down to its
acoustic equivalent on the same harmonic grid. A 2.4 GHz carrier folds 60 octaves down to
its base ratio. The audible result is the harmonic skeleton of the invisible field:
structural resonance tones, phase-alignment tones — the electromagnetic environment
rendered as music your ears already understand.

**Data** — The Chronos Slate maps the TimeGNN's extracted motifs (Ghost, Sparkle) across
a 97-day temporal buffer. While hands feel permittivity and ears hear heterodyned base
tones, eyes see the semantic structure: named patterns, phase progression, confidence, and
next-event ETA. The 128-byte `HeterodynePayload` struct carries all three channels across
the CPU/GPU boundary in a single Infinity Cache line, so no channel ever lags another.

This system is not a dashboard. It is an **instrument** — one that lets you touch, hear,
and see the electromagnetic field simultaneously, with each sense receiving a physically
accurate translation of the same underlying phenomenon.

### Development Philosophy

The Anakin C-3PO principle: centralize everything on the tethered workbench first. Sever
the tether only after the neural architecture is mathematically proven and compiles
flawlessly without mock stubs. Every track produces a runnable artifact. No track blocks
another unless explicitly stated in the dependency graph. Any algorithm that cannot run on
the sound card first has no business running on the Pluto+.

---

## Part II — Research Foundation

This section distills the key findings from the literature that justify each technical
choice in the architecture. It is not a literature review; it is a decision log. Each
finding is stated in terms of what it allows the system to do.

### 2.1 Wireless Radiation Field Gaussian Splatting (WRF-GS)

The core insight is that a wireless radiation field can be represented as a set of 3D
Gaussian primitives in exactly the same way that a visual scene is represented in 3D
Gaussian Splatting for real-time rendering. Wen et al. (2024, 2025) demonstrated that
this representation, combined with three RF-aware modules — a scenario representation
network, a projection model, and an electromagnetic splatting scheme — can synthesize
spatial spectra and channel state information at arbitrary positions in milliseconds after
training on relatively few RF measurements.

The key advantage over NeRF-based RF field representations is speed and compressibility.
WRF-GS produces results 100-1000× faster at inference time, and the learned representation
(a set of Gaussian parameters plus small neural network weights) is a compact object that
can be transmitted between nodes. This is why it serves as the system's "lightmap": the
Pluto+ trains a WRF-GS model of the room, and every downstream decision — null synthesis,
cloak waveform design, spatial correlation — queries that model rather than raw sensor
data.

WRF-GS+ (Wen et al., 2024) extends the base model with deformable Gaussians and
electromagnetic priors to capture fine multipath variations. SwiftWRF (Liu et al., 2025)
pushes inference further with 2D deformable Gaussians, exceeding 100,000 FPS on spectrum
synthesis. RF-3DGS (Zhang et al., 2024) demonstrated initializing Gaussian positions from
a digital-twin point cloud and training against both simulated and measured RF data.
GSRF (Yang et al., 2025) extends the framework to complex-valued Gaussians with
wavefront-based ray tracing for full-phase RF synthesis.

The frequency-embedding extension (Li et al., 2025) is directly relevant to the wideband
nature of this system: by conditioning each Gaussian's attenuation and intensity on
frequency as well as geometry, a single trained model handles the full range from sub-GHz
cellular to WiFi to millimeter wave without retraining. The EM attributes per Gaussian
(attenuation, emission) are learned functions of frequency and TX pose via small neural
networks, exactly the architecture used in this system.

**Consequence for architecture**: WRF-GS is not optional ornamentation. It is the
substrate that makes Track H (counter-waveform synthesis) physically correct rather than
numerically approximate. Without an accurate channel model, precoded waveforms that
"should" create nulls via multipath will instead create unpredictable constructive
interference. The Gaussian scene is the ground truth the TX optimization differentiates
through.

**Converting a 3D mesh to WRF-GS**: A 3D environment model with material properties
(concrete, wood, glass, wet) can be converted to a WRF-GS scene via a four-stage
pipeline. First, map visual/PBR material tags to RF electromagnetic parameters: complex
permittivity ε(f) and surface roughness scale from ITU-R P.2040 tables (Azpilicueta et
al., 2023). This mapping is not automatic — there is no off-the-shelf bridge from
"shiny/rough" BRDF tags to RF ε'. It must be specified or learned from joint optical and
RF measurements. Second, use a ray-launching EM simulator to generate a synthetic RF
dataset (RSS, CSI, power angular spectra) at many TX/RX poses from the annotated mesh.
Third, train a frequency-aware RF-GS model (wideband variant from Li et al.) against this
dataset. Fourth, the resulting Gaussian set is the compact RF digital twin deployable to
edge nodes.

The practical workflow for this project does not require building a mesh first. The Pluto+
and nRF24 array will collect real measurements from the actual room, and WRF-GS will be
trained directly on those measurements. The mesh-conversion pipeline is documented as a
future capability for pre-characterizing new environments before hardware deployment.

**Mesh shader acceleration**: WRF-GS already relies on GPU tile rasterization similar to
graphics pipelines. Mesh shaders and meshlets can accelerate Gaussian culling and spatial
partitioning — treating clusters of Gaussians as meshlets for coarse visibility culling
before detailed splatting, building bounding geometry for Gaussians in hardware, and
mapping splats to rasterized quads via hardware fragment stages. The core RF physics
(complex-valued radiance, EM priors, frequency embedding) remains in custom WGSL compute
shaders. Mesh shaders complement rather than replace RF-specific kernels.

### 2.2 Differentiable Ray Tracing for Radio Propagation

Ray tracing for radio propagation is the standard tool for site-specific channel modeling
in cellular, WiFi, and 6G systems (Yun & Iskander, 2015; Fuschini et al., 2015). In its
traditional form, it predicts path loss, delay profiles, and angles of arrival/departure
by launching ray bundles from a transmitter and computing reflections, diffractions, and
scattering through a 3D scene model.

**Sionna RT** (Hoydis et al., 2023) is the critical advance: it makes ray tracing
*differentiable* by computing gradients of channel impulse responses with respect to
material properties, antenna arrays, TX/RX positions and orientations, and RIS
configurations. This transforms ray tracing from a passive analysis tool into an active
component of an optimization loop. Given a desired field configuration at a target
location, gradient descent through the differentiable ray tracer finds the TX parameters
that produce it.

This is the mechanism behind Track H. The Pluto+'s transmission parameters are the
optimization variables. The WRF-GS scene is the differentiable channel model. The loss
function is field strength at the interference target (for nulling) or CSI deviation from
background (for cloaking). Sionna RT's architecture directly inspired the design of the
PINN loss wrapper in Track G4, which constrains the WRF-GS optimization to physically
realizable fields via Maxwell's equations as soft penalty terms.

For ISAC (Integrated Sensing and Communications) contexts, ray-traced channels feed joint
radar-communication waveform designs that discretize the angle domain for simultaneous
target tracking and data delivery (Li et al., 2021 on SS-OTFS). This informs the H-QAM
multi-layer packing approach: the same waveform carries navigation beacons (visible to
any receiver) and private control data (visible only at full SNR), using WRF-GS channel
knowledge to pre-compensate for the room's multipath.

For underwater acoustic channels, Hamiltonian space-time ray acoustics (Kaplun &
Katsnelson, 2025) treat time as an extra coordinate and explicitly model how
frequency-modulated signals evolve along acoustic ray paths, capturing modulation
distortion and time compression. This is the acoustic analog of the RF ray tracing in this
system and justifies using the same Daubechies wavelet substrate for both RF and acoustic
signal processing in Cyclone.

**Gap that justifies this project**: The literature contains no work on interactive
haptic interfaces driven directly by RF fields in a ray-traced, NN-optimized manner.
The building blocks exist in separate domains (differentiable EM propagation,
haptic rendering, real-time MRI, acoustic ray tracing) but have not been unified.
This project's novel contribution is exactly that integration.

### 2.3 Neural Reconstruction of Sensory Experience from EEG/EMG

The deep learning literature establishes that neural networks can decode both visual and
auditory experiences from brain signals. Visual reconstruction from EEG uses
CNN-GAN architectures and diffusion models to map EEG features to image representations
(Pan et al., 2024; Khare et al., 2022; Guo, 2024; Lan et al., 2023). The evidence for
this is rated strong by systematic review: multiple independent groups have achieved
recognizable reconstructions. Auditory reconstruction (music, speech identification from
EEG) uses LSTM, CNN, and transformer models and is rated moderate evidence — functional
but not yet high-fidelity (Daly, 2023; Bollens et al., 2025; Thornton et al., 2022).
Emotion recognition from EEG is rated strong evidence with high classification accuracy
across multiple architectures (Al-Qaysi et al., 2024; Jafari et al., 2023).

Multimodal systems combining EEG with EMG, EOG, and fNIRS consistently outperform
EEG-only systems (Lee et al., 2025; Li et al., 2025). This directly motivates Scarecrow
and Tin Man working as a collaborative pair rather than independent agents: when Scarecrow
decodes that the user is looking at an object while Tin Man decodes the user subvocalizing
its name, the combined evidence is substantially stronger than either alone.

Silent speech decoding via EMG (Gaddy & Klein architecture) uses a ResNet frontend for
muscle activation pattern extraction followed by a Conformer backend for phoneme/word
decoding, with an LLM scoring output for linguistic plausibility. The MONA disambiguation
pattern — where recent conversation context guides the decoder when EMG signal is
ambiguous between phonetically similar words — dramatically reduces word error rate.

The key practical constraint is individual variability: EEG models trained on one person
transfer poorly to another, and even within the same person, signal characteristics
change with hardware session variability, skin conductance, and fatigue. Large-scale
pretraining on foundation model EEG datasets (Jiang et al., 2024) and data augmentation
mitigate this, but the system must collect substantial real data from the specific user
before reaching reliable inference. The implication for this project is that Scarecrow and
Tin Man must operate in data-collection mode for extended periods before their decoding
accuracy is trusted for control decisions. The journal logs this progression explicitly.

Real-time, high-fidelity sensory reconstruction remains a research gap: most published
systems are offline. This is acknowledged as a limitation; the system's design degrades
gracefully to simpler emotion and command recognition in early deployment while working
toward the full reconstruction pipeline.

### 2.4 Distributed Multi-Modal Sensor Fusion and Edge AI

The architecture of this system (SDRs, mmWave, FPGAs, embedded controllers, IMUs)
mirrors what the distributed edge AI literature identifies as the canonical "heterogeneous
edge sensor network" pattern. The research consensus on making such systems coherent
identifies four layers of "glue" (Tang et al., 2023; Shuvo et al., 2023; Gill et al.,
2024):

The first layer is edge AI models running on constrained hardware via quantization and
pruning. This directly justifies the Coral TPU deployment of INT8 Normalizing Flow models
(Tracks A3 and I2) and the Pico 2 ESN reservoir classifier. The model is trained at full
precision on the main PC; inference runs at INT8 on the edge device with acceptable
accuracy degradation.

The second layer is federated learning for distributed model training where edge nodes
contribute local updates rather than raw data. In this project, the Pluto+'s onboard ARM
runs a pre-trained model and sends classification results back to the PC — the practical
variant of federated inference where the "communication round" is a scored FieldParticle
struct, not a gradient update. Full federated training is post-stabilization.

The third layer is mesh coordination via multi-agent reinforcement learning for
interference-aware resource scheduling (Zhang et al., 2021; Xu et al., 2021). This
motivates the Toto edge orchestrator's future role once the system scales to multiple
nodes. In the current single-node phase, the Toto Pi coordinates Coral, Pico 2, and
Pluto+ as a simple hub, not a full MARL system.

The fourth layer is middleware for cross-modal synchronization — aligning RF, audio, IMU,
and vision data streams with different sample rates and latencies. The `FieldParticle`
struct with its hardware-sourced `timestamp_us` field and `source` byte is this layer in
microcosm: every observation, regardless of its hardware origin, presents the same
interface with a traceable timestamp.

IRS/RIS and distributed beamforming (Pan et al., 2019; Ni et al., 2020) inform Phase J5,
where wall-mounted reconfigurable surfaces shape RF gradients into fields the user can
physically sense. This is post-Track-I territory but the theoretical basis is established.

### 2.5 Generative RF Scene Creation

The WaveVerse system (Zheng et al., 2025) is the closest published precursor to this
project's long-term vision: an LLM-guided 4D world generator that assigns dielectric
properties via language model reasoning and simulates phase-coherent RF fields for
imaging, sensing, and beamforming from text prompts. It is the clearest "treat RF like a
rendered medium" pipeline currently in the literature, though it does not yet use 3D
Gaussian representations.

RF Genesis (Chen & Zhang, 2023) uses diffusion models on visual scenes plus ray tracing
to generate mmWave sensing data from prompts. RF-Diffusion and RadioDiff (Chi et al.,
2024; Wang et al., 2024) treat RF time-frequency fields as the object of a diffusion
model — directly informing the Score-Based Diffusion approach in Track I2, where the
empty-room background is modeled as a manifold that the cloaking waveform projects the
biometric signature back onto.

iCOPYWAVES (Liaskos et al., 2022), RFGAN (Yu et al., 2021), and Wi-Fi holography (Holl &
Reinhard, 2016) represent the "RF-to-visual" direction: translating RF wavefronts into
XR-compatible views. This is the inverse of the visualization approach here (visual-to-RF
mapping), and the cross-modal encoding insights apply in both directions.

The end goal — LLM-guided, room-scale, holographic RF wave volumes that can be both
rendered and transmitted — would be a fusion of WaveVerse-style 4D generation with
physics-grounded RF-GS representation and XR-RF imaging ideas. That is Phase J territory.
The current roadmap builds the enabling infrastructure in Tracks A through I.

---

## Part III — Hardware Topology

All hardware operates as a unified local cluster until the NixOS edge deployment phase.
Do not deploy to standalone edge until the full pipeline is proven on Windows 11.

```
┌──────────────────────────────────────────────────────────────────────┐
│                      MAIN PC (Windows 11)                            │
│    RX 6700 XT + Ryzen 7 5700X + 64GB RAM (SAM/ReBAR enabled)        │
│                                                                      │
│  • Mamba / TimeGNN / LNN / PINN training  (Burn + wgpu DX12)        │
│  • WRF-GS 128-D Gaussian splat scene + hardware ray tracing         │
│  • Slint UI (Toto, Chronos, Hardware)                               │
│  • Orchestrates all edge devices; master of all model training       │
└──────┬──────────────────────┬────────────────────────┬──────────────┘
       │ USB                  │ USB-C or Ethernet       │ USB
       ▼                      ▼                         ▼
┌────────────────┐  ┌──────────────────────────┐  ┌──────────────────┐
│  Coral TPU     │  │  Pluto+ (ADALM-PLUTO+)   │  │  Pico 2 (RP2350) │
│                │  │  Zynq Z-7010/Z-7020      │  │                  │
│ INT8 inference │  │  Dual Cortex-A9 + FPGA   │  │ ESN reservoir    │
│ NF anomaly     │  │  Running Linux           │  │ classifier       │
│ calibration    │  │  TX/RX via libiio        │  │ 128-node INT8    │
│                │  │  Onboard IQ preprocess   │  │ hardware timestamps│
│                │  │  Lightweight onboard     │  │ Future: GPIO TX  │
│                │  │  inference (ESN/NF)      │  │ trigger (H4)     │
└────────────────┘  └──────────────────────────┘  └──────────────────┘
```

**RX 6700 XT specifics**:
- 12GB VRAM, RDNA2 architecture. All compute shaders `@workgroup_size(64,1,1)` — never 32,
  never 128. RDNA2 executes exactly 64-thread wavefronts; deviation wastes ALUs.
- SAM/ReBAR must be enabled (AMD CBS → NBIO → Above 4G Decoding + ReBAR Support).
  When active, the CPU addresses the full 12GB VRAM as a contiguous BAR. When disabled,
  only 256MB is addressable, destroying the zero-copy upload pipeline that Tracks G and H
  depend on. Verify before any GPU buffer work: log result to `assets/hardware_gate.txt`.
- 128-byte Infinity Cache line is the CPU/GPU boundary alignment unit. Every struct
  crossing that boundary must be exactly 128 bytes, padding counted with named heuristic
  fields, never `[u8; N]` dummies.

**Pluto+ specifics**: The Pluto+ is not a passive radio peripheral. It is a tethered Linux
dev board with an ARM CPU and Zynq FPGA. IQ pre-processing (band filtering, decimation,
basic feature extraction) runs on-device before bytes hit the bus, reducing host CPU load.
A trained ESN or NF model can be cross-compiled for ARM hard-float ABI and deployed via
SSH. The FPGA fabric can implement hardware FFT kernels in a future phase. Current
connection: USB-C or Ethernet — libiio handles both transparently. ADC/DAC depth: 12-bit
(AD9363). Theoretical SNR ceiling: ~72 dB. Practical: 50–60 dB in a real room.

---

## Part IV — System Architecture

### Agent Roster

**Dorothy** — The Mind (Orchestrator & Oracle)
Domain: Electromagnetic field analysis, WRF-GS synthesis, spectrum reasoning.
Core Model: Qwen3-VL (Vision) + DeepSeek-R1 (Reasoning). Memory: Personal journal
(Joplin), pattern library (Qdrant subset). MCP Tools: `analyze_rf_pattern`,
`generate_wrf_gs_field`, `query_dorothy_journal`, `propose_cleansing_strategy`.
Hardware: Desktop (RX 6700 XT for model inference).

**Glinda** — The Memory Engine
Domain: Multi-modal episodic memory, semantic graph, physics simulation.
Core Engine: Rust + Vulkan Compute. Protocol Layer: genkit-go MCP (thin wrapper).
Three memory tiers: (1) Sensory Buffer — 60s rolling window in lock-free ring buffers.
(2) Episodic Records — PostgreSQL + Qdrant vector embeddings. (3) Semantic Graph —
Neo4j or PostgreSQL recursive CTEs with typed edges (CAUSED_BY, CO_OCCURS_WITH,
PRECEDES, SPATIAL_NEAR). MCP Tools: `store_observation`, `query_temporal`,
`query_semantic`, `render_rf_hologram`, `wrf_trainer`.

**Scarecrow** — The Brain Interface
Domain: EEG/BCI, neural signal decoding, cognitive state monitoring.
Core Tech: CNN-GAN for visual reconstruction, LSTM/Transformer for auditory decoding,
emotion classification trained on DEAP/SEED datasets. MCP Tools: `decode_visual_perception`,
`decode_auditory_perception`, `get_emotional_state`, `correlate_with_environment`.
Hardware: Muse/Emotiv EEG → Brick Road → Cyclone → Crystal Ball (Coral TPU inference).

**Tin Man** — The Voice
Domain: EMG silent speech, gestural control, subvocalization.
Core Tech: ResNet-Conformer (Gaddy & Klein architecture) + MONA LLM disambiguation.
MCP Tools: `decode_silent_speech`, `recognize_gesture`, `get_articulation_state`.
Hardware: Surface EMG → Brick Road → Cyclone → Crystal Ball.

**Lion** — The Defender
Domain: Autonomous cleansing, jamming detection, interference mitigation.
Autonomy: High (autonomous with cognitive oversight). MCP Tools: `detect_interference`,
`execute_cleanse`, `flag_anomaly`, `get_cleansing_status`. Hardware: Raspberry Pi + Coral.

**Crystal Ball** — The Inference Engine
Domain: Shared ML model serving, real-time inference substrate.
Core Tech: ONNX Runtime + MIGraphX (RX 6700 XT), TensorFlow Lite (Coral TPU),
model versioning and A/B testing. MCP Tools: `run_inference`, `get_model_status`,
`compare_model_versions`.

**Wizard** — The Controller
Domain: Gestural input, spatial wave manipulation, haptic feedback.
Core Tech: IMU fusion (MediaPipe vision + controller gyro/accel), Extended Kalman Filter,
DTW gesture recognition (Vulkan compute). MCP Tools: `record_gesture`,
`translate_to_field_config`, `render_haptics`.
Hardware: Switch Joy-Con/PS5 controller → SDL3.

**Oz** — The Visualizer
Domain: Real-time 3D rendering, particle systems, Gaussian splatting.
Core Tech: wgpu (WebGPU/Vulkan), bevy_gaussian_splatting.
Capabilities: bee swarm particle rendering (60 FPS), WRF-GS volumetric splatting,
multiple viz modes (swarm/slice/trajectory/heatmap), glassless 3D via eye tracking.
Hardware: RX 6700 XT.

**Emerald City** — The Translator
Domain: Acoustic-harmonic color mapping, spatial correlation.
Algorithm: Logarithmic frequency-to-hue (F4 = violet anchor), amplitude → lightness,
bandwidth → saturation, phase coherence → lightness modifier.
Capabilities: synesthetic color assignment, TDOA/RSSI spatial fusion, CSI extraction
following WiGrus methodology. Phase coherence Γ(r) = |ΣᵢEᵢ(r)| / Σᵢ|Eᵢ(r)|.

**Cyclone** — The Signal Engine
Domain: Universal signal processing substrate.
Core Tech: Lock-free ring buffers (Rust), VkFFT (GPU) / SIMD FFT (CPU),
Daubechies wavelet transforms (6 levels), CSI preprocessing (PCA denoising per WiGrus).
Processes RF, acoustic, and biosignal streams through the same abstractions.

**Brick Road** — The Hardware Abstraction
Domain: Unified WaveSource interface for heterogeneous sensors.
Core Trait: `get_coordinate_frame()`, `stream_signal()`, `get_calibration()`,
`get_telemetry()`. Backends: Pluto+ SDR, Switch controller, mmWave, camera, EEG/EMG.
Supports hot-swapping, device discovery, calibration management.

**Kansas** — The Interface
Domain: TUI/GUI system state, diagnostics, control surfaces.
Desktop: ratatui (terminal, 60 FPS). Edge: KMS/DRM framebuffer (OpenGL ES).
Design: contextual disclosure, event-driven updates, calm technology principle.

**Toto** — Edge Device Orchestrator
Role: Raspberry Pi edge coordinator, future drone swarm controller.
Current: coordinate Coral, Pico 2, Pluto+. Future: federated learning coordination,
autonomous drone deployment. Generic Toto → specific names (R2, etc.) as fleet scales.

### The Dependency Graph

```
FieldParticle struct ─────────────────────────────────┐
        │                                              │
        ▼                                              ▼
SignalIngester trait                    tokens.slint (design language)
        │                                              │
   ┌────┴────┐                              ┌──────────┴──────────┐
   │         │                              │                     │
   ▼         ▼                              ▼                     ▼
AudioIn   RF/SDRIn                    Toto widget         Chronos widget
   │         │                              │                     │
   └────┬────┘                              └──────────┬──────────┘
        │                                              │
        ▼                                              │
 Mamba inference loop ◄───────────────────────────────┘
  (128-D latent embeddings)
        │
        ├─────────────────────────────────┐
        │                                 │
        ▼                                 ▼
 TimeGNN + LNN (Track B)        WRF-GS scene (Track G)
 [128-D embeddings as nodes]    [128-D embeddings as splat color]
        │                                 │
        │                         PINN loss wrapper (G4)
        │                                 │
        └─────────────┬───────────────────┘
                      │
                      ▼
             Ray Tracing TX pipeline (Track H)
             [multipath null steering, DPC, extreme QAM]
                      │
                      ▼
             Biometric Cloak (Track I)
             [E(3) + Diffusion + LNN]
```

Everything above Mamba can be developed in parallel.
Everything below it is sequential — each layer requires the one above.

---

## Part V — Invariant Rules

These rules apply to every file, every agent, every track. A violation is a build failure,
not a warning.

**The Forensic Rule** — This system is forensic infrastructure. Fake data is evidence
tampering. If real data is unavailable, the pipeline halts and logs why. No demo mode, no
synthetic fallback, no animated sine waves. Every timestamp traces to hardware source.
`SystemTime::now()` is not acceptable for `FieldParticle.timestamp_us`.

**The Variable Backend Rule** — Every algorithm that produces or consumes a waveform must
be written against the `SignalBackend` trait, not against a specific hardware interface.
Hardcoding a backend inside algorithm code is a build failure equivalent to `todo!()`.

The three required backends — all must compile from day one, even if two are stubs:
`Backend::Audio` (24-bit sound card via CPAL/WASAPI — always first test target),
`Backend::Pluto` (AD9363 via libiio — second target after Audio is green),
`Backend::File` (write IQ/PCM to disk — always available, required as side effect of
every test run, committed alongside the test result).

No algorithm milestone is complete until verified against at least `Backend::Audio` and
`Backend::Pluto`. `Backend::File` output must always be generated. The soundcard is the
proving ground. The Pluto+ is the deployment target.

**The 128-Byte Law** — All structs that cross the CPU/GPU boundary must be exactly
128 bytes (one RX 6700 XT Infinity Cache line). Padding fields must be named active
heuristics, never `[u8; N]`. Add `const _: () = assert!(std::mem::size_of::<T>() == 128);`
immediately after every such struct definition.

**The Wave64 Mandate** — All WGSL shaders use `@workgroup_size(64, 1, 1)`. Never 32.
Never 128. This is a hard RDNA2 architectural requirement.

**The Timestamp Rule** — On Windows, hardware timestamps use `QueryPerformanceCounter`
via `windows-sys`, not `std::time::Instant`. Store `(current_qpc - epoch_qpc) /
(freq / 1_000_000)` as `timestamp_us`. The session epoch QPC is captured once at
process start. This is what ETW uses and what the forensic corpus requires.

**The Pre-Flight Rule** — Every agent writes this block at the top of its first new file:

```rust
// === PRE-FLIGHT ===
// Task:           [Track X, Milestone Y]
// Files read:     [list]
// Files in scope: [list]
// Acceptance:     [verbatim from roadmap]
// Findings:       [relevant patterns observed in existing code]
// === END PRE-FLIGHT ===
```

**The No-Mock-In-Production Rule** — Test files referenced during a programming test must
never be used in the real program. The file ingester must check that a file is not in the
`tests/` or `examples/` directory before using it as a real data source.

**Other hard rules**: `todo!()` and `unimplemented!()` are compilation failures — not
warnings. Mock data in UI tracks must animate and be physically plausible — not zeros,
not constants. Every new `.slint` file imports `ui/tokens.slint`, uses `Colors.*` not hex
literals. ROCm and Vulkan ray tracing are post-stabilization — do not use on any current
track. Do not modify files outside stated scope without flagging the conflict first.

---

## Part VI — FieldParticle Canonical Definition

```rust
/// One observation from one sensor at one moment in time.
/// Exactly 128 bytes — one RX 6700 XT Infinity Cache line.
/// Every field is either a physically meaningful quantity or a pre-computed
/// heuristic derived from real sensor data. No padding bytes allowed.
#[repr(C)]
pub struct FieldParticle {
    pub timestamp_us:               u64,        //  8  QPC-sourced microseconds from process epoch
    pub freq_hz:                    f64,        //  8  center frequency of observation
    pub energy:                     f32,        //  4  normalized signal strength, 0.0–1.0
    pub phase_coherence:            f32,        //  4  Γ: 0.0 = null, 1.0 = constructive
    pub position_xyz:               [f32; 3],   // 12  spatial estimate in meters
    pub material_id:                u8,         //  1  octave bucket 0–11 (Emerald City hue class)
    pub source:                     u8,         //  1  0=AudioHost,1=PlutoOnboard,2=HostProcessed,3=Pico
    pub _pad0:                      [u8; 2],    //  2  alignment pad — reserved for future flag bits
    pub doppler_shift:              f32,        //  4  pre-computed heuristic: radial velocity estimate
    pub phase_velocity:             f32,        //  4  pre-computed heuristic: wavefront speed estimate
    pub scattering_cross_section:   f32,        //  4  pre-computed heuristic: effective scatter area
    pub bandwidth_hz:               f32,        //  4  spectral width of this observation
    pub anomaly_score:              f32,        //  4  Coral NF output; 0.0 if Coral unavailable
    pub motif_hint:                 u8,         //  1  last ESN classification from Pico; 255=unknown
    pub _pad1:                      [u8; 3],    //  3  alignment pad — reserved
    pub embedding:                  [f32; 16],  // 64  first 16 dims of 128-D Mamba latent (CPU summary)
                                                //     full 128-D lives in GPU-side storage buffer
}
// 8+8+4+4+12+1+1+2+4+4+4+4+4+1+3+64 = 128 bytes exactly
const _: () = assert!(std::mem::size_of::<FieldParticle>() == 128);
```

The `freq_to_material_id` function maps any frequency to its octave bucket (0–11) using
logarithmic frequency space. The anchor is F4 (349.23 Hz) → bucket 5 (violet). Octave
equivalents produce the same bucket. All four doc-tests must pass:
`freq_to_material_id(440.0)` → 9, `freq_to_material_id(880.0)` → 9,
`freq_to_material_id(349.23)` → 5, `freq_to_material_id(2_400_000_000.0)` → document
exact value.

---

## Part VII — Track Specifications

### Phase 0 — Foundation (Complete First, Unblocks Everything)

Duration: 1–3 days. Zero external dependencies.

**0-A: FieldParticle + SignalIngester**
Files: `src/ml/field_particle.rs`, `src/dispatch/signal_ingester.rs`,
`src/dispatch/audio_ingester.rs`, `src/dispatch/rf_ingester.rs`.
Acceptance: `cargo test --doc ml::field_particle` passes all four doc-tests.
Blocks: Tracks A, B, C, D, E, F, G.

**0-B: Design Language Tokens**
Files: `ui/tokens.slint`, `assets/SKILL-SLINT-MD3.md`.
Acceptance: `slint-viewer ui/tokens.slint` shows no errors. Every color token matches
`SKILL-SLINT-MD3.md §2`.
Blocks: Tracks E and F.

**0-C: Hardware Gate — SAM Verification**
Query `adapter.get_info()` in wgpu and verify the BAR size shows ~12GB, not 256MB.
Log result to `assets/hardware_gate.txt`. If SAM is disabled, enable it in BIOS
(AMD CBS → NBIO → Above 4G Decoding + ReBAR Support) before any Track G or H work.
Acceptance: Result documented. Does not block other tracks but must precede G1.

**0-D: Hardware Configuration Applet**
File: `ui/hardware.slint` (and supporting Rust backend).
Delete: all examples, tests, and `.slint` files except `examples/toto` and `ui/toto.slint`.
Content: Material Design 3 hardware cards for RTL-SDR, Pluto+, and Soundcard. Each card
opens a configuration panel with a full radio tuner (frequency input with unit selector,
RX/TX toggle, tone generator, sinc interpolation mode, W-OFDM burst mode). Transmit
button sends a continuous wave at the specified frequency to the selected backend. The
panel wires to `Backend::Audio`, `Backend::Pluto`, and `Backend::File` via the
`SignalBackend` trait.
Acceptance: Hardware applet launches, all three device cards render correctly. Setting
Pluto+ to 1 MHz TX and pressing Transmit produces a measurable carrier in SDR++ at
1 MHz. Setting Soundcard to 440 Hz tone plays a sine wave through the default output
device. Changing to W-OFDM mode produces a visually distinct waveform in the spectrum
analyzer. [MOCK] badge appears on any control that is not yet wired to real hardware.
Blocks: Nothing immediately — this is the user-facing hardware setup UI for the whole
project.

---

### Track A — Mamba Inference Loop

Depends on: 0-A. Independent of: all UI tracks, ROCm, Vulkan.

Goal: `Drive`, `Fold`, `Asym` vary dynamically from real FieldParticle input. The Mamba
model's 128-D latent embeddings are the universal data currency — they feed WRF-GS splats
(Track G), TimeGNN nodes (Track B), and edge filters.

**A1 — Dispatch loop wiring**
Remove the 9× audio repeat hack from `src/main.rs`. Wire: `AudioIngester::ingest()` →
sort by energy → accumulate 4096 unique samples → `Mamba::forward()` → 
`project_latent_to_waveshape()` → Drive/Fold/Asym.
`project_latent_to_waveshape()` takes the 128-D latent and produces three scalars via
three learned linear projections (three separate weight vectors of length 128,
dot-producted with the latent). Drive = energy magnitude projection. Fold = harmonic
clustering projection. Asym = left-right spectral asymmetry projection.
Acceptance: Run 60 seconds, log Drive/Fold/Asym every second. Values must differ from
each other and change over time. Constant output = failure.

**A2 — RF ingester integration**
Add `RFIngester` to dispatch loop alongside `AudioIngester`. Pluto+ optional at this
stage — `RFIngester` can read a pre-recorded IQ file (raw interleaved f32 complex,
little-endian, no header, extension `.iq` or `.cf32`). Verify file size is divisible by 8.
If the file has an unrecognized header, halt and log what was found — do not guess.
Acceptance: material_id distribution shows both audio-range (0–4) and RF-range (5–11)
clusters active in the same 10-second window.

**A-HET — Heterodyning to acoustic base ratios**
For each dominant FFT bin, compute `f_audio = f_rf / 2^N` landing in 20–1000 Hz. This
is identical to the hue computation — route the result to a sine oscillator and play
through `Backend::Audio`. Multiple bins produce a chord. `Backend::File` writes the
synthesized PCM continuously as a timestamped audit log (non-negotiable). The timestamp
in the filename uses the QPC session epoch — named `session_YYYYMMDDTHHMMSS.pcm`.
Acceptance: With a known 2.4 GHz WiFi AP active, audio output plays a continuous tone
at the correct folded frequency (verify with a frequency analyzer). When AP is off, tone
disappears. Tone pitch changes as Pluto+ retunes, tracking the RF scan in real time.

**A-EC — Phase Coherence (EMERALD CITY extension)**
Compute Γ(r) = |ΣᵢEᵢ(r)| / Σᵢ|Eᵢ(r)| per-bin across FFT output. Map to HSL:
hue = frequency (existing octave mapping, unchanged), lightness = Γ, saturation =
inverse bandwidth. A WiFi null (standing wave zero) appears as dark violet, not absent.
Γ also feeds G-SPH2 as the RF repulsion field — highly coherent regions repel SPH
particles to field boundaries.
Acceptance: Introduce a known standing wave (Pluto+ pointed at a metal surface at
measured distance). Visualization shows a dark band at the theoretical null distance.
Verify null position matches λ/2 prediction within 5%.

**A-WOFDM — Wavelet OFDM Transmit Synthesis**
WGSL compute shader: IDWT synthesis of W-OFDM symbols on the GPU. Input: frequency-domain
symbol vector. Output: time-domain IQ/PCM waveform, continuous, no guard intervals.
Daubechies wavelets have compact support — no ringing, no inter-symbol interference, no
cyclic prefix needed. The 20% OFDM guard interval overhead disappears.
`SignalBackend` selects destination — never hardcoded in the shader.
Acceptance (Backend::Audio first): W-OFDM symbol count in a fixed time window must exceed
standard OFDM by ≥ 20%. Then promote to Backend::Pluto: same test via TX→RX loopback,
result must match Audio within measurement noise. Backend::File written for both runs.

**A3 — Edge filter deployment**

*Coral TPU*: Deploy quantized Normalizing Flow for anomaly probability calibration. NF
learns the probability density of the empty room's baseline. Anomaly score = distance
from that density. Writes to shared `AtomicU32` continuously. Unplugging Coral degrades
gracefully — score freezes, no panic.
Acceptance: Anomaly score updates independently of main Mamba loop.

*Pico 2 (Embassy, no_std)*: 128-node ESN reservoir classifier (128×128 INT8 = 16KB,
input weights 128×8 INT8 = 1KB, readout weights = const array trained offline on main PC).
Classifies each 1ms window independently. Sends results via USB CDC with hardware
timestamps from RP2350 hardware counter (independent of PC clock — this is the whole
point). If 128-node reservoir exceeds SRAM budget: fall back to 64-node, document.
Acceptance: ESN classifies at ≥ 1kHz. Hardware timestamps appear in host log. Unplug
degrades gracefully.

*Pluto+ ARM*: Cross-compile lightweight ESN or NF for ARM hard-float (armhf). Deploy to
Pluto+ filesystem via SSH/SCP. Model runs on IQ data locally; sends FieldParticle-
equivalent structs to host (not raw IQ). Host log shows `source: PlutoOnboard` distinct
from `source: HostProcessed`.

---

### Track B — TimeGNN + LNN Backend

Depends on: 0-A, A1. Independent of: Track F widget, wgpu, ROCm.

Goal: TimeGNN produces named motifs, confidence scores, and forward predictions. Liquid
Neural Networks handle variable-rate temporal dynamics, accommodating dropped SDR packets,
variable audio buffer sizes, and simultaneous tracking of microsecond RF bursts and
12-hour thermal cycles.

**B1 — Trainer stabilization**
`train_timegnn()` uses real FieldParticle embeddings from
`@databases/forensic_logs/events.jsonl`. If file is empty or absent: log error and halt.
Never fall back to synthetic. `pattern_discovery.rs`: silhouette score ≥ 0.6 gate enforced.
Acceptance: 10 epochs, checkpoint to `checkpoints/timegnn/epoch_010.pt`. Log shows at
least one "rejected: score X.XX" line.

**B2 — Hot-swappable configuration bridge**
Temperature τ, prediction horizon, attention window, motif minimum support adjustable at
runtime via TOML file watcher, no restart required.
Acceptance: τ change 0.14 → 0.80 takes effect within one epoch, edge count increases.

**B3 — Motif output stream**
TimeGNN emits `MotifEvent { name: String, phase: u8, phase_total: u8, confidence: f32,
next_event_eta_secs: Option<f32>, freq_hz: f64 }` to a channel consumable by Track F.
Names are adjective-noun (GHOST, SPARKLE) — not UUIDs.
Acceptance: Mock stream produces a MotifEvent every ~10s with cycling names and plausible
phase progressions. Real model replaces mock in the same session.

**B4 — NT-Xent loss exposure**
Loss as `AtomicU32` (f32::to_bits()), history in lock-free ring buffer (120 values).
Acceptance: Loss trends from ~2.1 toward ~0.05. Ring buffer readable from separate thread.

**B5 — Liquid Neural Network integration (Burn crate)**
Replace fixed-step sequence layer with CfC LNN: `dx/dt = -x/τ + f(x, input, t)·Δt`.
Use actual elapsed time between FieldParticle observations from `timestamp_us` differences
in real seconds — do not assume uniform timesteps.
Use Burn crate from B5 onward. PyTorch/tch-rs acceptable for B1–B4 if already in flight;
do not retrofit, do not start new training code in PyTorch after B4 is green.
Acceptance: Feed TimeGNN a sequence with 30% of steps randomly dropped. LNN prediction
error ≤ 15% above baseline. Fixed-step RNN degrades significantly — document both.

---

### Track C — Glinda Memory Engine

Depends on: 0-A. Independent of: all UI tracks, GPU rendering.

**C1 — Episodic record layer**
PostgreSQL schema: `observations(id, timestamp_us, sensor_type, material_id, energy,
position_xyz, description_text, embedding_vec)`. Qdrant: `synesthesia_observations`,
384-dim (all-MiniLM-L6-v2 on CPU). MCP tools: `store_observation`, `query_temporal`,
`query_semantic`, `mark_significant`.
Acceptance: Store 1000 observations. `query_semantic("WiFi interference")` returns 5 most
relevant in under 100ms.

**C2 — Sensory buffer layer**
Lock-free ring buffer (Rust) holding 60 seconds of FieldParticles. Memory-mapped
interface so Cyclone writes without blocking. Background goroutine promotes interesting
windows to episodic records.
Acceptance: Sustains 192kHz × 512 particles/frame with zero dropped samples over 60s.

**C3 — Semantic graph layer**
Neo4j or PostgreSQL recursive CTEs. Typed edges: CAUSED_BY, CO_OCCURS_WITH, PRECEDES,
SPATIAL_NEAR.
Acceptance: After 24h of observation, graph contains ≥ 3 entity types and ≥ 2 discovered
relationship types.

---

### Track D — Dorothy Cognitive Loop

Depends on: 0-A, C1. Independent of: WRF-GS rendering, ray tracing, any widget.

**D1 — Autonomous loop (LangGraph)**
State machine: Wake → Observe → Compare → Analyze → Hypothesize → Document → Sleep.
Notes have YAML frontmatter: `{ timestamp, sensor_ids, glinda_obs_ids, confidence, tags }`.
Acceptance: Run 3 hours unattended. Journal shows 3 new notes each referencing real Glinda
observation IDs.

**D2 — Reflection process**
Weekly synthesis note: "Observed X instances of Y pattern, suggesting Z."
Acceptance: Synthesis note produced after injecting 5 notes sharing a common tag.

**D3 — MCP interface**
`analyze_rf_pattern(obs_id)`, `query_dorothy_journal(query)`,
`get_dorothy_opinion(situation)`, `propose_cleansing_strategy(pattern_id)`.
Acceptance: `get_dorothy_opinion` returns a response citing ≥ 1 specific journal entry
by date.

---

### Track E — Toto Widget (Mamba Applet)

Depends on: 0-A, 0-B. Reads from: Track A. Independent of: Tracks B, C, D, F.

**E1 — Static widget with mock data**
`ui/toto.slint` importing `ui/tokens.slint`. Three zones: header (TOTO + anomaly score +
Neural Auto-Steer toggle), oscilloscope canvas (glowing waveform + cluster label),
telemetry strip (Drive/Fold/Asym tiles with progress bars). Wave color cycles: red (60Hz)
→ teal (85kHz) → violet (2.4GHz) every 2s.
Acceptance: Opens within 3s, all zones animate, color transitions smooth (400ms),
[MOCK] badge visible, zero `todo!()` in compiled code.

**E2 — Live data wiring**
Replace MockDataStream with real channel from Track A dispatch loop. Windows 11 DWM
Acrylic blur via `DwmSetWindowAttribute(DWMSBT_TRANSIENTWINDOW)`.
Acceptance: Wave color responds to actual dominant frequency. Drive/Fold/Asym differ from
each other and change with audio input.

**E3 — WASM build**
`cargo build --example toto --target wasm32-unknown-unknown` succeeds. No
`std::thread::sleep`, `std::fs`, or `SystemTime` in WASM paths.
Acceptance: Loads in Chrome/Edge. [MOCK] badge shows. Animations run.

---

### Track F — Chronos Widget (TimeGNN Applet)

Depends on: 0-B, E1 complete. Reads from: Track B. Independent of: Tracks C, D, wgpu.

**F1 — Static widget with mock data**
`ui/chronos.slint` importing `ui/tokens.slint`. Three zones: header (CHRONOS + τ slider),
prediction graph canvas, telemetry strip (NT-Xent Loss sparkline / Motif name+phase /
Next Event countdown). Graph: teal nodes (past) → violet (present) → red (predicted).
Acceptance: Opens within 3s, motif name changes, countdown ticks, [MOCK] badge visible.
Side by side with Toto: same instrument family aesthetic.

**F2 — τ control wiring**
Temperature slider adjusts τ in real time (0.05 → 2.0, logarithmic scale).
Acceptance: Moving slider from 0.14 to 0.80 causes visible edge density increase within
one animation frame.

**F3 — Settings flyout**
Sections: Exploration vs Detection dial, Temporal Scales, Online Learning. Freeze toggle
greys out learning rate and forgetting rate controls. Flyout slides in from right.
Acceptance: Flyout opens/closes with animation. Freeze toggle disables sibling controls.

**F4 — Live data wiring**
Connect to Track B MotifEvent channel and NT-Xent loss ring buffer. τ slider writes back
to Track B hot-swap bridge (B2).
Acceptance: Real motif events appear. Changing τ in widget changes training behavior
within one epoch.

---

### Track G — WRF-GS Scene + PINN Wrapper + Sub-Tracks

Depends on: A1 (Mamba 128-D embeddings), E2 (wgpu DX12 proven in widget).
Independent of: Tracks B, C, D, F.
Platform: Windows 11, wgpu DX12. ROCm is explicitly post-stabilization.

**G2-RDNA2 (Amendment to all G shaders)**: `@workgroup_size(64, 1, 1)` always. SoA layout
for any buffer > 10k elements. All CPU/GPU boundary structs 128 bytes, named padding.

**G1 — Static Gaussian splat render**
`src/rendering/wrf_gs_renderer.rs`. 1000 mock Gaussians colored by material_id → RGBA,
instanced draw, Gaussian opacity falloff in fragment shader.
Acceptance: `cargo run --example oz_preview` at 60 FPS. Camera orbits.

**G2 — 128-D embedding splats**
Replace RGB color with 128-D Mamba embedding in GPU-side buffer. Visualization: project
embedding → RGB via small learned linear layer for display; full 128-D used by ray tracer
and PINN. Scale to 10k Gaussians.
Acceptance: Gaussians respond to live audio. Dominant WiFi signal produces violet cluster
at correct normalized position. ≥ 30 FPS at 10k Gaussians.

**G3 — Wavelet Radiance Field decomposition**
Extend each Gaussian with Daubechies wavelet components at 6 decomposition levels.
A single splat models both acoustic-band and RF-band interactions simultaneously.
Acceptance: Splat at a hard wall shows high-frequency RF reflection AND low-frequency
acoustic absorption with distinct opacity values at each scale.

**G-SPH1 — SPH density pass**
Each particle queries neighbors in a spatial hash grid. Kernel: Müller 2003 poly6 density,
spiky pressure gradient. Spatial hash: Kogge-Stone prefix scan inside a 64-thread
workgroup (LDS only, no global atomics). O(1) neighbor lookup.
Acceptance: 1M SPH particles, density pass ≤ 2ms GPU time. Density in uniform sphere
matches analytical result within 1%.

**G-SPH2 — PBD constraint solve**
Incompressibility, surface tension, RF-field-driven repulsion (particles repelled from
high-Γ regions, clustering at field boundaries). PBD preferred over SPH pressure
projection: unconditionally stable.
Acceptance: Stable at dt = 16.67ms, no explosions or tunneling. Strong WiFi AP causes
visible particle clustering at constructive-interference boundary.

**G-SPH3 — Compute-to-indirect vertex pulling**
GPU-driven draw: compute pass culls and writes surviving indices to DrawIndirectArgs
buffer. Vertex stage pulls AetherParticle from storage buffer via vertex_index. Zero
CPU-side draw call overhead.
Acceptance: DrawIndirectArgs populated without CPU readback. Particle render adds ≤ 2ms
to G3 frame time.

**G-RB1 — Complex Fresnel equations**
Replace all Schlick approximations with full complex Fresnel using ε_c = ε' - jε''.
Material library from ITU-R P.2040: dry concrete, wet concrete, glass, wood, metal,
human body.
Acceptance: Dry concrete at 2.4 GHz (ε' ≈ 5.0, ε'' ≈ 0.17) matches ITU-R P.2040
Table 3 within ±1 dB.

**G-RB2 — RF-GGX microfacet distribution**
Roughness α_RF = σ_surface / λ_RF. Transition from specular to diffuse follows Rayleigh
criterion: σ > λ / (8·cos θᵢ).
Acceptance: Same concrete surface produces near-specular at 2.4 GHz (α_RF ≈ 0.01) and
diffuse at 60 GHz (α_RF ≈ 0.25). Verify transition matches published scattering data.

**G-RB3 — Double-Debye wetness model**
ε_eff(f, S) = ε_dry + S·Δε_Debye(f). Surface wetness S adjustable at runtime.
Acceptance: At 2.4 GHz, dry wood (ε' ≈ 2.0) vs wet wood (ε' ≈ 20–30) produce
measurable reflected power difference matching published data.

**G4 — PINN loss wrapper**
Wrap WRF-GS optimization with PINN loss term embedding Maxwell's equations (RF) and the
acoustic wave equation as soft constraints at collocation points.
Acceptance: Attempt to optimize toward a physically impossible configuration. PINN loss
term grows by ≥ 100× and optimization halts or redirects before producing invalid result.

**G5 — BVH acceleration structure** (prerequisite for Track H)
Build BVH over Gaussian scene via wgpu ray tracing feature flag. Rebuild when mean
Gaussian position displacement > 0.05 normalized units.
Acceptance: BVH build under 5ms for 10k Gaussians. Ray intersection returns correct
nearest Gaussian in under 0.1ms.

---

### Track H — Ray Tracing TX Pipeline

Depends on: G5 (BVH proven), A1 (Mamba embeddings), B2 (prediction coordinates).
Do not begin until G5 is confirmed and passing.

**H1 — Ray casting through WRF-GS**
Cast N rays from Pluto+ antenna position through BVH to target. Record path length,
bounce count, Gaussian wavelet properties per ray.
Acceptance: Path length histogram for 1000 rays shows multipath delay spread 1–50 ns for
room-sized environment.

**H2 — Phase delay accumulation**
Per ray: total phase = Σ(path_length_i / λ) mod 1.0. Identify constructive vs destructive
paths at target.
Acceptance: For a known simple geometry (single reflector), phase calculation matches
analytical solution within 5 degrees.

**H3-DPC1 — Costa precoding (Tomlinson-Harashima Precoding)**
Input: desired received waveform at target, WRF-GS channel matrix H. Output: pre-coded TX
waveform x such that H·x ≈ desired at target. THP uses a simple modulo operation at the
TX to approximate exact Dirty Paper Coding (which requires exponential codebook search).
DPC theorem: with non-causal knowledge of interference, TX achieves same capacity as if
interference did not exist. In this system, the "interference" is room multipath — the
Pluto+ has exact non-causal knowledge via WRF-GS. The room is part of the codec.
Acceptance: In single-reflector test environment, THP-precoded TX produces ≥ 6 dB SNR
improvement at target vs naive IFFT TX.

**H3-DPC2 — Room-as-codec null synthesis**
Synthesize waveform whose bounce paths destructively interfere at body-occupied volume,
constructively everywhere else. The room's walls do the work; Pluto+ provides the seed.
Acceptance: Null depth at target volume ≥ 15 dB with 10× lower TX power than naive
counter-waveform approach.

**H-QAM1 — Constellation depth profiling**
Transmit sweep of QAM orders (64 → 256 → 1024 → 4096) via Pluto+ loopback or second
SDR. Plot EVM vs QAM order to find practical ceiling for the specific room.
Acceptance: Identify highest stable QAM order where EVM < -30 dB. Document as the
"room's QAM ceiling."

**H-QAM2 — Multi-layer symbol packing**
Encode three independent streams in a single symbol using QAM depth layers: Layer 1
(coarse — any receiver, navigation/beacon), Layer 2 (medium — 12-bit receiver,
WRF-GS field updates), Layer 3 (fine — full SNR only, private channel).
Acceptance: Three decoders at different precision thresholds each correctly recover their
layer from a single transmitted symbol stream.

**H-FRAC1 — GPU IDWT fractal synthesis shader**
WGSL compute: accepts macro symbol vector + micro symbol vector, synthesizes a single
IQ/PCM waveform containing both. Daubechies-4 macro layer (bits 12..7), Daubechies-8
micro layer (bits 6..1). Orthogonal decomposition: the two wavelet families do not
interfere with each other's data. To a 6–8 bit receiver, the micro layer is invisible
inside the quantization noise floor.
Acceptance (Backend::Audio first): Macro = sub-100 Hz tone, micro = 18–22 kHz ultrasonic
channel. Both on same PCM stream. Verify micro layer recovered correctly; standard decoder
cannot see it. Then Backend::Pluto: same structure on RF IQ, macro visible to 6-bit
equivalent decoder, micro invisible.

**H4 — Pico 2 TX trigger** (post-stabilization)
Pico 2 receives waveform buffer via USB CDC, fires Pluto+ trigger via GPIO at precisely
calculated time using PIO state machine as master clock.
Acceptance: Timestamp jitter under 50 μs over 1000 transmissions.
Status: Do not begin until H3 is proven and NixOS migration is complete.

---

### Track HA — Haptic Sub-System (600Hz)

Depends on: G-SPH2 (physics running), G-RB3 (RF-BSDF roughness available).
Hardware: Voice Coil Actuators (exact model TBD).

**HA1 — Localized 600Hz PBD haptic solve**
Only particles in the controller's bounding box re-solved at 600Hz. Dedicated CPU thread,
3-buffer async readback ring (never `wgpu::Maintain::Wait`). Budget: 1.67ms total per
haptic frame. Haptic thread pinned to a dedicated CPU core, isolated from render thread.
Acceptance: Haptic readback does not spike the 60 FPS render frame time. VCA updates at
verified 600Hz.

**HA2 — LF/HF bifurcation**
Low-frequency (< 80 Hz): SPH pressure gradient magnitude. Interpretation: "how much RF
energy is concentrated here." VCA amplitude ∝ |∇P_SPH|.
High-frequency (80–300 Hz): RF-GGX roughness α_RF and Double-Debye ε_eff at proxy
location. Targeted at Pacinian corpuscles (200–300 Hz peak sensitivity). WiFi off
concrete (α_RF ≈ 0.01) feels categorically different from WiFi off foam (α_RF ≈ 0.3).
Acceptance: Two surfaces of known different RF roughness produce distinct VCA HF content.
Difference perceptible in blind A/B test.

**HA3 — Stochastic resonance envelope**
RF-BSDF roughness σ controls noise envelope on haptic signal. Rough materials → noisy
haptic envelope. Smooth materials → clean tones.
Acceptance: Smooth metal → clean 120 Hz tone. Rough concrete → same 120 Hz tone with
measurable broadband noise added.

---

### Track I — Biometric Cloak (Capstone)

Depends on: All of Tracks A–HA at final milestones, NixOS migration complete.
Do not begin until the full system is proven and ROCm-optimized.

**I1 — Biometric signature characterization (E(3)-Equivariant Networks)**
Model how a human body perturbs baseline acoustic and RF fields, rotation-invariant.
TimeGNN achieves > 90% accuracy: "person present" vs "room empty" from RF alone, no
cameras.

**I2 — Presence null synthesis (Score-Based Diffusion)**
The Normalizing Flow from Track A3 has learned the explicit probability density of the
empty room's background. RF-Diffusion's reverse process synthesizes a waveform that
projects the biometric signature back onto the "unremarkable environmental RF" manifold.
Counter-waveform creates a null in the body-occupied volume; breathing and heartbeat
modulations actively cancelled as they appear.
Acceptance: ≥ 15 dB reduction in body-induced CSI variation vs no cloak, measured by
external SDR receiver.

**I3 — Adaptive forgetting for cloak stability (LNN)**
LNN from Track B5 provides continuous-time dynamics for smooth state transitions during
movement. Learned time constants τ per node naturally handle different rates of breathing
modulation, footstep transients, and slow postural drift.
Acceptance: Walk slowly across room while cloak is active. External receiver CSI variation
stays below threshold throughout movement.

**I4 — Optical cloak** (Conceptual only)
UV LED array tuned to exploit adversarial perturbations in camera classifiers.
Status: Legal review required before any implementation. Do not implement.

---

### Phase J — Post-Track-I Extension Tracks

These require the full A–HA pipeline proven and stable.

**J1 — RF Proprioception**: Continuous RF body schema state vector. Belt-mounted
mmWave + Pluto+ maps how the body blocks and diffracts. Output: slow continuous haptic
pattern across torso/arm VCA array. Test: find a router with eyes closed using only RF-
body haptic gradient.

**J2 — Sub-Nyquist Spectral Retina**: Compressed sensing with randomized LO hops.
Sparse recovery via IHT/OMP. Mamba as learned denoiser post-recovery. 100× faster
wide-band attention in exchange for exact waveform fidelity.

**J3 — RF-Texture-Smell Chain**: Complex permittivity classification → material label →
scent profile LUT. Petrichor for wet porous (ε' ≈ 25), metallic for high σ. Hardware:
Escents-class wearable scent device.

**J4 — Ambient Backscatter Interaction Surfaces**: RF energy-harvesting backscatter tags
on objects. Touch changes backscatter resonance on ambient WiFi carriers. Zero-power
physical controls in space.

**J5 — RIS-Driven Haptic Fields**: Wall-mounted RIS with controllable phase. Optimize
to create sharp spatial RF gradients ("RF ridges") at user position. Novel direction:
nobody is currently using RIS to craft fields whose goal is human touch.

**J6 — Differentiable Calibration Loop (Burn autograd)**: Port RF-BSDF math (Fresnel,
RF-GGX, Double-Debye) into Burn tensors. Loss = predicted scattering − measured from
Pluto+/mmWave. Gradient descent over ε', ε'', α_RF, water saturation S. Brief
calibration step when system is idle.

**J7 — Field Compass UX**: Sub-Nyquist spectral retina scans for strongest RF pockets.
RF proprioception distributes direction-coded haptic gradient at waist/wrists. "Compass
needle made of vibration." Test protocol: RF navigation trials, eyes closed.

---

### Phase K — Impulse Radio / rpitx Experiments

Prerequisites: Track H complete, Pluto+ TX pipeline proven.
Legal note: rpitx transmits on frequencies requiring licensing. All experiments in a
shielded environment or under Part 15 / amateur radio authorization.

**K1 — rpitx W-OFDM proof-of-concept**: CPU IDWT on Pi, transmit via rpitx, receive via
Pluto+. Acceptance: EVM < -20 dB over 1 meter in shielded environment.

**K2 — Impulse radio ranging**: Very short pulses (< 1 ns equivalent via rpitx frequency
hopping). Measure time-of-flight to reflectors. Compare against WRF-GS geometry.
Acceptance: Measured reflector distances match WRF-GS scene within ±10 cm.

**K3 — Multi-node fractal mesh**: Two Toto Pi nodes each transmitting macro beacon +
micro data. Pluto+ receives both, separates by wavelet basis. Acceptance: Both micro-layer
streams decoded with < 5% symbol error despite overlapping frequencies.

---

## Part VIII — Shannon-Hartley Context

For reference, where each technique sits against the Shannon limit at the Pluto+'s
practical operating point (50 dB SNR, 1 MHz bandwidth, C = B·log₂(1+SNR) ≈ 16.6 Mbits/s):

Standard 64-QAM OFDM with CP achieves ~4 bits/s/Hz (~80% of theoretical, CP wastes 20%).
W-OFDM at 64-QAM achieves ~5 bits/s/Hz (~100% of theoretical for this SNR).
W-OFDM at 4096-QAM (achievable at 55 dB SNR) achieves ~10 bits/s/Hz.
Fractal W-OFDM (macro + micro layers) targets ~12–14 bits/s/Hz at 12-bit hardware.
Dirty Paper Coding adds no bits/s/Hz but removes multipath SNR penalty — equivalent to
6–10 dB SNR improvement in a reflective room, translating to ~2–3 extra bits/symbol.
The full stack targets ~10–12 Mbits/s in a 1 MHz window — within factor of 2 of Shannon.

---

## Part IX — Technology Stack

| Track | Language | Key Libraries | GPU |
|-------|----------|---------------|-----|
| 0-A | Rust | rustfft, bytemuck | No |
| 0-B | Slint DSL | — | No |
| 0-D | Rust + Slint | slint 1.15, CPAL, windows-sys | No |
| A | Rust | burn 0.21, wgpu 28 | Optional |
| A3-Pluto | Python or Rust (armhf cross) | numpy / burn-no-std | No (ARM) |
| A3-Pico | Rust (Embassy, no_std) | embassy-usb, fixed-point math | No (MCU) |
| B | Rust | burn 0.21, petgraph | Optional |
| C | Rust + Go | genkit-go, qdrant-client, tokio-postgres | No |
| D | Python | langgraph, ollama, joplin-api | No |
| E | Rust + Slint | slint 1.15, windows-sys | No |
| F | Rust + Slint | slint 1.15 | No |
| G | Rust | wgpu 28, WGSL | Yes (DX12) |
| H | Rust | wgpu 28, nalgebra | Yes (DX12) |
| HA | Rust | CPAL for VCA output, wgpu | Yes |
| I | Rust + Python | all of the above | Yes |

**Graphics Architecture — Vulkan + wgpu Hybrid**:
Glinda WRF-GS Renderer uses pure Vulkan (ash) for hardware ray tracing:
`VK_KHR_ray_tracing_pipeline`, `VK_EXT_mesh_shader`, `VK_KHR_acceleration_structure`.
Oz Visualization uses wgpu for particle swarm rendering (stable mesh shader support in
v28.0+, cross-platform compatibility). Shared Vulkan textures via
`wgpu::Texture::create_from_hal::<wgpu::hal::api::Vulkan>()` for zero-copy RF field →
visualization interop. Migration path: `#[cfg(feature = "wgpu-rt")]` when wgpu ray
tracing matures.

---

## Part X — Neural Architecture Reference

| Model | Track | Purpose | Target |
|-------|-------|---------|--------|
| UnifiedFieldMamba | A | Feature extraction, 128-D embeddings | GPU |
| TimeGNN | B | Temporal pattern graph, motif discovery | GPU |
| LNN (CfC) via Burn | B5 | Variable-rate temporal dynamics | GPU |
| Normalizing Flow | A3, I2 | Anomaly calibration; empty-room baseline | Coral TPU |
| Echo State Network | A3 | Fast first-pass classifier | Pico 2 + Pluto+ ARM |
| NT-Xent Contrastive | B | Motif similarity (temperature τ) | GPU |
| PINN | G4, H3 | Maxwell/wave equation constraints on TX | GPU |
| all-MiniLM-L6-v2 | C | Observation text embeddings for Glinda | CPU |
| LangGraph + LLM | D | Dorothy cognitive loop | CPU |
| E(3)-equivariant Net | I1 | Body-field perturbation, rotation-invariant | GPU |
| Score-Based Diffusion | I2 | RF-Diffusion: background manifold synthesis | GPU |

---

## Part XI — ROCm / NixOS Migration Gate

Do not migrate until all of the following are true:
- [ ] A1: Drive/Fold/Asym confirmed varying from real particle input
- [ ] E2: Toto widget live on Windows 11 with real data
- [ ] G1: WRF-GS render at 60 FPS on Windows 11 DX12
- [ ] B1: TimeGNN checkpoint saves without error
- [ ] 72 hours of continuous operation on Windows 11 without crash

Migration adds: ROCm HIP backend, Vulkan ray tracing, KWin compositor blur,
cooperative group kernels for progressive BVH refinement.
Migration changes nothing: APIs, Slint components, track structure, Pluto+ libiio
interface — all stay identical. DX12 → Vulkan is a backend swap, not a rewrite.

---

## Part XII — Agent Assignment Protocol

For every task assigned to a coding agent:

1. Specify exact track and milestone (e.g., "Track A, Milestone A2")
2. List files to read before writing any code
3. List files in scope — changes outside this list require flagging before proceeding
4. Copy acceptance criteria verbatim from this document
5. Instruct the agent: if you encounter Phase 0 items (missing const assertions,
   missing token imports, hardware gate file) in files you are already touching,
   complete them in the same pass. Small 0-phase fixes do not require a separate task.
   If it takes under five minutes and is in a file you are already editing, fix it.

---

## Part XIII — Hardware Dependency Map

```
Track              Win11  Pluto+ direct  Pluto+ via Pico  Pico 2  Coral  VCA
───────────────    ─────  ─────────────  ───────────────  ──────  ─────  ───
0-A                  ✓
0-B                  ✓
0-C                  ✓
0-D                  ✓       optional
A-1                  ✓
A-2                  ✓       optional
A-WOFDM              ✓       ✓
A-EC                 ✓
A3 Coral             ✓                                              ✓
A3 Pico              ✓                                      ✓
A3 Pluto             ✓       ✓
B-1 to B-5           ✓
C-1 to C-3           ✓
D-1 to D-3           ✓
E-1 to E-3           ✓
F-1 to F-4           ✓
G-1 to G-SPH3        ✓
G-RB1/2/3            ✓       optional
G4 PINN              ✓
G5 BVH               ✓
H-1                  ✓
H-2                  ✓
H3-DPC1/2            ✓       ✓
H-QAM1/2             ✓       ✓
H-FRAC1              ✓       ✓
H4 Pico trigger                              ✓               ✓
HA1/2/3              ✓                                                     ✓
I-1                  ✓       ✓                               ✓      ✓
I-2                  ✓       ✓                                      ✓
I-3                  ✓       ✓                                      ✓
J1–J7 (post-I)               ✓                               ✓     ✓     ✓
K1–K3 (post-H)       ✓       ✓
```

---

## Part XIV — Current Status

| Track | Status | Blocking issue |
|-------|--------|----------------|
| 0-A FieldParticle | 🔴 Not started | Do first — unblocks everything |
| 0-B tokens.slint | 🔴 Not started | Do first — unblocks E, F |
| 0-C SAM gate | 🔴 Not started | Do before G1 |
| 0-D Hardware applet | 🔴 Not started | Needs 0-B; runs alongside others |
| A1 Dispatch loop | 🟡 Partial — audio hack present | Needs 0-A |
| A2 RF ingester | 🔴 Not started | Needs A1 stable |
| A-HET Heterodyning | 🔴 Not started | Needs A1 FFT per-bin |
| A-EC Phase coherence | 🔴 Not started | Needs A1 FFT output per-bin |
| A-WOFDM W-OFDM | 🔴 Not started | Needs A1, Pluto+ TX confirmed |
| A3 Coral NF | 🔴 Not started | Needs A1 stable |
| A3 Pico ESN | 🔴 Not started | Needs A1; SRAM budget TBD |
| A3 Pluto ARM | 🔴 Not started | Needs A2; cross-compile env TBD |
| B1 TimeGNN | 🟡 Partial — stubs in train/load | Needs 0-A, real corpus |
| B5 LNN (Burn) | 🔴 Not started | Needs B1 stable |
| C1 Glinda episodic | 🔴 Not started | Needs 0-A schema |
| D1 Dorothy loop | 🔴 Not started | Needs C1 |
| E1 Toto static | 🟡 Design proven in React | Needs 0-B, Slint translation |
| E2 Toto live | 🔴 Not started | Needs E1, A1 |
| F1–F4 Chronos | 🔴 Not started | Needs E1, 0-B |
| G1 WRF-GS static | 🔴 Not started | Needs A1, E2 (wgpu proven) |
| G-SPH1/2/3 | 🔴 Not started | Needs G1 pipeline |
| G-RB1/2/3 | 🔴 Not started | Needs G3 |
| G4 PINN | 🔴 Not started | Needs G-RB (channel matrix) |
| G5 BVH | 🔴 Not started | Needs G4 |
| H series | 🔴 Not started | Needs G5 |
| HA1/2/3 Haptics | 🔴 Not started | Needs G-SPH2, VCA hardware |
| H4 Pico TX trigger | 🔴 Post-stabilization | Needs H3 + NixOS |
| I Biometric cloak | 🔴 Conceptual only | Needs A–HA + NixOS |
| J1–J7 | 🔴 Post-Track-I | Full pipeline proven |
| K1–K3 rpitx | 🔴 Post-Track-H | Legal gate |

---

*This document is the single source of track status, dependency truth, and research
justification for Project Synesthesia. Update the status table when milestones complete.
Do not add tracks without updating the dependency graph in Part IV. Do not contradict
the invariant rules in Part V. Agent instructions in Part XII apply to all sessions.*
