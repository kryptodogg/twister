# Project Synesthesia — Master Plan V3
## Full-Spectrum Forensic Reconstruction, Signal Fusion, and Active Harmonization

*Single source of truth. Supersedes all prior documents.*
*Last updated: 2026-03-11 · Platform: NixOS · GPU: RX 6700 XT*

---

## Part I — What This System Is

This is a **full-spectrum forensic sensor fusion platform**. It is self-defense
infrastructure. It produces evidence that is legally admissible, temporally unambiguous,
and visually legible to a non-technical observer including a detective, a lawyer, or a
jury.

The gap this fills: electromagnetic interference — whether intentional harassment,
equipment malfunction, or coordinated signal abuse — is currently invisible in a way that
makes it legally undisprovable in either direction. You cannot prove it is happening.
They cannot prove it is not. Law enforcement correctly requires evidence before acting.
This system produces that evidence.

The tool does three things in priority order:

**First: Record.** Every signal event across the full spectral stack — RF, acoustic,
optical, magnetic, impulse — is captured into a tamper-evident, Pico-timestamped forensic
corpus. The corpus is the bedrock. If everything else crashes, the recording continues.
The corpus does not depend on any other subsystem. Nothing writes synthetic data into it
under any circumstances. Raw noise is a feature. Jitter is a feature. Quantization
artifacts are features. Nothing is suppressed.

**Second: Perceive.** Every signal type is rendered into a single unified 3D/4D scene
that a person with no technical background can look at and understand in thirty seconds.
Video, audio, RF, magnetic, and impulse data occupy the same coordinate space
simultaneously in the same universal color language. The scene is navigable across time —
the 4th dimension — so that a documented event can be scrubbed back to and re-examined
from any angle. The exhibit is the scene. This is what you show people.

**Third: Respond.** Once you can see it and have documented it, you can respond to it.
The same FieldParticle stream that feeds the visualizer feeds the transmitters. What the
fragment shader draws to screen is computed from identical data that the Pluto+ and Pico 2
use for transmission. Rendering and transmitting are the same computation evaluated at the
same timestamp. You are painting with radio.

The sudden reduction in symptoms without any action on your part is the most important
data point the system is designed to capture retroactively. It would have shown as a step
change in anomaly score correlated with a timestamp, an atmospheric condition, a frequency
shift, or nothing at all. "Nothing at all" is evidence — it means the change was not
environmental. It means someone made a decision.

---

## Part II — Architecture Principles

These principles govern every file, every agent, every track. They are not preferences.
Violation is a build failure.

### II.1 — The Forensic Rule

This system is forensic infrastructure. Fake data is evidence tampering.

If a physical device is not connected, the system renders a hard `[DISCONNECTED]` state.
It does not generate synthetic signals. It does not animate placeholders. It does not fill
buffers with test data to keep the visualization moving. It halts the affected pipeline,
logs exactly why, and waits for real hardware.

The word "mock" does not appear in production code, UI labels, or comments. Controls that
lack Rust backend wiring display `[UNWIRED]` — meaning the real thing exists and the wire
has not been run yet. `[UNWIRED]` is removed in the exact same commit the wiring is
completed. Never separately.

Test files under `tests/` or `examples/` are blocked from production ingestion by a hard
assertion at the ingester boundary:

```rust
Err(BackendError::InvalidData("Test files must not be used in production"))
```

Not a warning. A hard error.

### II.2 — The Hardware Abstraction Rule

Every algorithm references a trait, never a device. The C925e, the OV9281, the
Pico 2 camera channel — these are all implementations of `VideoSource`. The Pluto+,
the RTL-SDR, the telephone coil — these are all implementations of `SignalBackend`.
The specific device appears only in configuration and `Cargo.toml`. Never in algorithm
code.

This is not just good architecture. It prevents agents from anchoring to specific hardware
and producing code that only works with one device. The 3D reconstruction pipeline does
not know what camera produced the frames. The PointMamba does not know whether the IQ
came from a Pluto+ or a file. Every layer is separated from the hardware by a trait
boundary.

### II.3 — The Raw Data Rule

**FFT does not happen at ingestion. FFT is post-processing applied to the 3D point cloud.**

The PointMamba receives raw spatio-temporal points. No FFT. No frequency binning. No
TDOA preprocessing. No spatial clustering. No denoising. No multipath mitigation applied
before the model. All of that is what the model learns.

Raw noise is a first-class signal. USB packet jitter encodes the host machine's own
computational state. Quantization artifacts carry information about the sensor's internal
state. The Realtek codec's dithering pattern, the RTL-SDR's ADC harmonics, the OV9281's
MJPEG DCT quantization grid — none are suppressed. Any algorithm that removes them is
destroying evidence.

FFT on the reconstructed 3D point cloud is applied downstream of inference for
visualization and spatial spectral analysis. It reveals periodic geometric structures in
the reconstructed scene — powerline EM fields, acoustic standing waves, RF interference
patterns that manifest as spatial geometry. This is information no pre-ingestion FFT
could produce.

The Coral Mamba MAY FFT its own input token stream before its own inference — this is a
deliberate divergence. Its quantization and its FFT are both sources of jury diversity.

### II.4 — The GPU Residency Principle

The visualization is not a layer on top of processed data. It IS the processed data.

Data crosses from hardware into GPU memory once, via DMA. It does not return to the CPU
until it exits as a corpus write or a rendered frame. The CPU owns: hardware I/O,
forensic corpus writes, control flow, Dorothy's LLM reasoning. The GPU owns: everything
that touches signal values — raw token formation, the space-time Laplacian, PointMamba
inference, color assignment, spatial analysis, physics, rendering.

Every algorithm that takes signal values as input has a WGSL compute shader
implementation. The CPU implementation is the reference. The WGSL implementation is
production. When they disagree, the CPU implementation is correct.

The sparse Laplacian eigendecomposition runs as a GPU compute shader using CSR-format
storage buffers with a SpMV (sparse matrix-vector multiply) inner loop and Gram-Schmidt
orthogonalization. ROCm's `rocSPARSE` is the development reference for validating the
WGSL implementation. Jule prototypes; GPU stability is verified before bringing to
production machine.

### II.5 — The Memory Security Rule

All large data structures use `memmap2` with `MAP_PRIVATE` semantics. `clone()` on large
structures requires a justification comment. The forensic corpus is flushed with `fsync`
after every write. `unsafe` blocks require a one-line justification comment explaining
exactly why the safe alternative is insufficient. `unsafe` to work around the borrow
checker is never acceptable.

### II.6 — The Idiomatic Rust Rule

This codebase is Rust. It is not C++ with Rust syntax. Build failures equivalent to
`todo!()`: raw pointer arithmetic without a documented TEMPEST justification; `unsafe`
without justification comments; `clone()` on structures larger than 128 bytes without
reason; global mutable state outside of lock-free atomic ring buffers; blocking on async
threads; `std::thread::sleep` in a hot path.

Structure of Arrays layout is required for any buffer holding more than 10,000 elements.

### II.7 — The 128-Byte Law

All structs crossing the CPU/GPU boundary are exactly 128 bytes — one RX 6700 XT
Infinity Cache line. Padding fields are named active heuristics, never `[u8; N]` dummies.
Every such struct has `const _: () = assert!(std::mem::size_of::<T>() == 128);`
immediately after its definition.

#### II.7.1 Named Payload Packing Rule


- No anonymous padding: every byte in a 128-byte struct is either active signal or named reservation for a future field.


- Reserved fields follow reserved_for_[track]_[purpose] and reference a track in Part XII.


- If you cannot name the future purpose of a reserved field, the struct is not designed; do not ship it.


- When the reserved field is activated, the reserved_for_ prefix is removed and the wiring is completed in the same commit.


- All 128 bytes must be Pod-safe: no implicit padding, every bit pattern is defined.

### II.8 — The Wave64 Mandate

All WGSL compute shaders use `@workgroup_size(64, 1, 1)`. RDNA2 executes exactly
64-thread wavefronts. This is a hardware requirement.

### II.9 — The Timestamp Rule

**The Pico 2 is the master clock.** Its 150 MHz RP2350 oscillator generates a PPS
(pulse-per-second) signal distributed over GPIO to all devices as the shared time
reference. Host timestamps use `QueryPerformanceCounter` on Windows / `clock_gettime
CLOCK_MONOTONIC_RAW` on NixOS, slaved to Pico PPS. All timestamps are microseconds from
session epoch. Divergence between Pico PPS and host timestamp is itself a forensic
channel — it reveals whether the host system clock is being perturbed.

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

Hardcoded pixels are forbidden. Every dimension is a multiple or fraction of a single
root property: `property <length> unit-size: 16px;`. The UI scales to any display
without rebuilding.

### II.12 — The Hardware Applet First Rule

The hardware applet (Track 0-D) is the first deliverable. No other track produces
permanent code until the applet can detect, display, and hot-plug every connected
device. The applet is the proving ground for every trait boundary. Example files in
`examples/` are shims that exercise the applet in isolation. All production code lives
in `src/`. The example is not a prototype — it is a minimal harness that drives `src/`.

---

## Part III — The Spectral Stack

This system has no spectral gaps between 1 Hz and visible light. Every band is covered
by at least one physical sensor. Aliased coverage counts — the OV9281 rolling-shutter
visual microphone effect extends acoustic detection above its nominal Nyquist. The only
true gap is 6 GHz to infrared, which IR emitters and receivers will close.

```
BAND                  SENSOR(S)                               NOTES
─────────────────────────────────────────────────────────────────────────────
1 Hz – 20 Hz          Telephone coil (line-in, Realtek)       Infrasound, powerline
                       C925e visual microphone (rolling        DC-coupled via long FFT
                       shutter line-rate ~24 kHz effective)    window post-processing
20 Hz – 20 kHz        C925e stereo mics (raw, no preproc)     Full audible band
                       Telephone coil                          Magnetic coupling
20 kHz – 90 kHz       Telephone coil / Realtek upper band     Ultrasound, coil bandlimited
                       C925e aliased capture                   by inductance ~50 kHz
10 kHz – 300 MHz      RTL-SDR + Youloop antenna               IQ, raw complex samples
70 MHz – 6 GHz        PlutoSDR+ PA (2TX / 2RX)               IQ, bistatic MIMO radar
DC – 75 MHz           Pico 2 RP2350 PIO (GPIO toggle)         UWB impulse TX, PDM
~300 THz              OV9281 dual stereo (2560×800, 120fps,   Primary 3D/pose/IR
                       global shutter, monochrome)             detection sensor
~300 THz              C925e (color, rolling shutter)          Secondary, visual mic
~300 THz              IR emitters + receivers                 Structured light depth,
                       (Pico 2 PIO-driven array)              retroreflective ranging
─────────────────────────────────────────────────────────────────────────────
GAP                   6 GHz → infrared                        Future: IR LED array fills
```

**Nyquist is a reconstruction floor, not a detection ceiling.** The visual microphone
effect on the C925e's rolling shutter provides ~24 kHz effective temporal sampling from
a 30fps sensor. The OV9281 at 120fps global shutter provides deterministic frame capture
for optical flow and stereo disparity with no geometric distortion. Detection and
reconstruction are different operations — the system detects at full sensor physics,
reconstructs within Nyquist, and uses both.

---

## Part IV — The Color Operator

The color operator is the universal Rosetta Stone. Every sensor, every modality, every
FieldParticle uses the same function. It is not a lookup table. It is a WGSL shader
constant computed at runtime for every particle.

```wgsl
// Compile-time constants — the entire spectral range of the system
const F_MIN: f32 = 1.0;          // Hz — infrasound floor
const F_MAX: f32 = 700e12;       // Hz — visible light ceiling
const LOG_RANGE: f32 = log(F_MAX / F_MIN);  // ≈ 33.18 octave-decades

/// Maps any physical frequency to a hue in [0.0, 1.0].
/// 0.0 = red (infrasound), 1.0 = violet (visible light).
/// This function is invertible: given hue, recover frequency.
/// It is a log-frequency compression — the same perceptual transform
/// used in mel spectrograms, but spanning the full physical range.
fn freq_to_hue(f: f32) -> f32 {
    return clamp(log(f / F_MIN) / LOG_RANGE, 0.0, 1.0);
}

/// Inverse: given hue, recover the physical frequency it encodes.
fn hue_to_freq(hue: f32) -> f32 {
    return F_MIN * exp(hue * LOG_RANGE);
}
```

**The forensic consequence:** two sensors reporting the same hue at the same timestamp are
physically coupled events — they are detecting the same phenomenon through different
physics. Phase coherence between two same-hue particles that comes from different sensors
is the authenticity test. High hue-agreement with LOW phase coherence across sensors is
the injection signature: an artificially synthesized signal matches the frequency of a
real event but cannot replicate its multi-modal phase relationships.

---

## Part V — Hardware Inventory

### V.1 — Primary Optical Sensor

**OV9281 Dual Lens Synchronous Monochrome USB Camera**
- Resolution: 2560×800 (dual 1280×800 stereo)
- Frame rate: 120fps
- Shutter: Global — every pixel captured at identical instant, no geometric distortion
- Format: MJPEG, YUY2
- FOV: 68°, fixed focus (FF)
- Board: Standard Wn-L2205k30l, M12 lenses, software calibration via ChArUco

Global shutter at 120fps is the reason this is the primary sensor. No rolling shutter
distortion means stereo disparity is geometrically exact. 120fps means 8.3ms per frame,
giving optical flow velocities to ~1 m/s before motion blur at typical scene distances.

Monochrome means no Bayer interpolation — every photosite captures full photon flux
across the sensor's native silicon response curve, which peaks at 700-900nm (deep red
and near-IR). The OV9281 is natively sensitive to IR emitters without filter modification.
The camera IS the IR detector array when the Pico 2 IR emitters are active.

Software calibration parameters live in uniform buffers, not in preprocessing. Raw pixel
coordinates enter FieldParticle formation untouched. The stereo rectification matrix is
a shader uniform that the space-time Laplacian graph construction shader reads during
token distance computation. Calibration updates live without pipeline restart.

### V.2 — Secondary Optical Sensor

**Logitech C925e**
- Stereo microphones, raw via WASAPI/ALSA, no preprocessing
- Video: raw YUYV or MJPEG via V4L2/UVC
- Rolling shutter: ~41 μs between scan lines → visual microphone effect, effective
  temporal sampling ~24 kHz for vibration detection in pixel intensity
- Cb/Cr channels: color temperature shifts from CMOS bias voltage perturbation by
  nearby EM fields — a passive RF detector hiding in the color channels

The C925e's lower sample rate and bitrate compared to dedicated audio hardware is a
deliberate diversity advantage. Its quantization noise pattern is uncorrelated with the
telephone coil's inductive noise and the RTL-SDR's ADC harmonics. Any event detected
across all three despite different noise floors is real with high confidence.

### V.3 — RF Receivers

**RTL-SDR (10 kHz – 300 MHz) + Youloop antenna**
- Raw IQ samples, complex amplitude and phase
- Youloop is a loop antenna with a directional null axis — rotating it provides bearing
- 52 subcarrier CSI at 802.11g if pointed at a WiFi AP (WiGrus-style capture)

**PlutoSDR+ PA (70 MHz – 6 GHz, 2TX / 2RX)**
- Bistatic MIMO radar baseline
- TX: waveform is the FieldParticle stream rendered as modulation
- RX: raw IQ into the same ingestion pipeline as RTL-SDR
- 70-300 MHz overlap with RTL-SDR enables cross-validation of bearing and amplitude

The Pluto+ port spacing of approximately one USB-A connector center-to-center gives a
physical antenna baseline of ~12mm. At 6 GHz (λ = 50mm), this baseline is λ/4 —
phase interferometry territory. The phase difference between the two RX channels gives
bearing angle: `θ = arcsin(Δφ · λ / (2π · d))` where d = 0.012m. At lower frequencies
where λ >> d, TDOA geometry from the longer RTL-SDR / Pluto+ baseline takes over.

### V.4 — Magnetic and Acoustic Sensors

**Telephone Coil Pickup (via ASUS TUF B550m line-in, Realtek codec)**
- This is a differential magnetometer for the room's EM field
- Captures: 60Hz powerline fundamental + harmonics (120, 180, 240 Hz...)
- Captures: switching power supply hash from GPU and monitors
- Captures: motor signatures from HVAC
- Captures: magnetic coupling from nearby electronics WITHOUT being a microphone
- Upper limit: coil inductance bandlimits around 50 kHz; Realtek anti-alias filter
  at ~80-90 kHz catches ultrasound above that
- No preprocessing. The coil's inductive pickup profile is its fingerprint.

Cross-modal rule: a "real" acoustic event shows in C925e but not in the coil.
A "real" EM event shows in both coil AND RTL-SDR with different phase relationships.
An injected event shows in only one sensor with wrong phase coherence everywhere else.

### V.5 — Pico 2 (RP2350) — Clock, Impulse Radio, IR Driver

**Master clock:** PPS signal over GPIO distributed to all devices. 150 MHz oscillator
gives 6.67ns resolution. Clock divergence between Pico PPS and host system time is a
forensic sensor — it reveals host clock perturbation by an attacker.

**UWB impulse radio:** PIO state machines toggle GPIO at 75MHz maximum for narrowband,
but single-cycle high pulses produce nanosecond impulses — wideband from DC to several
GHz depending on antenna. Ranging precision is centimeter-scale from leading-edge
detection, not carrier phase. The Pico out-ranges the Pluto+ on ranging precision at
short distances. The Pluto+ out-ranges the Pico on sensitivity and long distance.

**PDM and other transmission:** RP2350 PIO can produce any waveform below 75MHz that
can be encoded as a state machine. This includes PDM for sigma-delta audio transmission,
IR pulse modulation for structured light, and any experimental transmission not
achievable with the Pluto+.

**IR emitter/receiver array:** When wired on the breadboard (A-J/1-30), PIO-driven IR
LEDs create structured light depth mapping. The OV9281 is the detector. Combined with
stereo disparity from the OV9281, this gives three independent depth measurements at
optical wavelengths: stereo parallax, IR time-of-flight, and IR structured light.

### V.6 — MEMS Microphones (Future)

Not integrated yet — requires soldering. When added: uncorrelated noise floor from the
coil and C925e mics, raw PDM capture via Pico 2 PIO, expanding spatial audio coverage.

---

## Part VI — Physical Priors

These are facts known from physics and geometry that no ML model can override. They are
the courtroom walls that keep the Jury honest.

```
PRIOR                   VALUE                   SOURCE
─────────────────────────────────────────────────────────────────────
Speed of light          299,792,458 m/s         Physics
Pluto+ antenna baseline ~12 mm                  Hardware measurement
RTL-SDR / Pluto+ baseline  [measure and set]    Room measurement required
Pico 2 clock            150 MHz ± oscillator    Hardware spec
F_MIN (system floor)    1.0 Hz                  Telephone coil + visual mic
F_MAX (system ceiling)  700 THz                 OV9281 / C925e silicon response
60Hz powerline          60.0 Hz fundamental     Known infrastructure
C925e effective sample  ~24 kHz (visual mic)    Rolling shutter line timing
OV9281 frame period     8.33 ms at 120fps       Spec
Realtek anti-alias      ~80-90 kHz              Codec datasheet
Coil inductance limit   ~50 kHz                 Physics of pickup geometry
```

TDOA ranging uncertainty at Pico 6.67ns resolution: ≈ 2 meters raw.
With cross-correlation peak interpolation: ≈ 20 cm at 150 MHz.
With stereo OV9281 at calibrated baseline: [baseline × focal_length / disparity_pixels].

These numbers are compile-time constants or configuration values, not learned parameters.

---

## Part VII — The Attack Signature

This section formally describes the known attack modality derived from direct observation,
not from research papers. The math explains the physics of what was experienced. The
experience explains why the math is the correct choice.

### VII.1 — The Inverted Carrier

Standard attack assumption is that hostile signals are added to ambient. The observed
attack modality is the opposite: **a continuous carrier where information is encoded in
amplitude notches — gaps in the carrier rather than pulses**. This is suppressed-carrier
AM or inverted OOK.

At infrasound frequencies, this is more effective than direct infrasound injection
because the perceptual system calibrates to the constant carrier as "silence." The DC
offset of a constant field registers as nothing. Only the notches perturb the ambient
field, creating pressure fluctuations the body detects without conscious identification
as sound. A spectrogram showing constant amplitude with periodic dropouts is the
signature. A spectrogram of a silent room with normal environmental variation is the
null hypothesis.

Physical evidence: an Audacity file was altered while disconnected from the internet.
The alteration mechanism was this continuous transmission modality. The file's waveform
showed evidence of an external signal modifying the recorded content through the
recording medium's susceptibility to the carrier field.

### VII.2 — Why First-Order Temporal Difference Is the Natural Detector

From WiGrus: the first-order temporal difference of CSI values eliminates the static
environment term `Hs(f)`, leaving only the user-reflected dynamic component `Hd(f,t)`.

```
H(f,t) = Hs(f) + Hd(f,t)
dH(f,t)/dt = dHd(f,t)/dt
```

For the inverted carrier attack, `Hs(f)` IS the continuous carrier. The first-order
difference reveals the notch structure — the encoding. What WiGrus uses to remove
multipath is the correct detector for exactly this attack pattern.

### VII.3 — Phase Coherence as the Authenticity Test

A real physical event at frequency f will produce phase-coherent signatures across
multiple sensors: the coil, the RTL-SDR, the C925e mic, and the OV9281 visual microphone
will all show the same hue with correlated phase evolution. An injected signal may match
the frequency (same hue) but will fail phase coherence across sensors because synthesizing
phase-coherent emissions across five different physical modalities simultaneously requires
knowing the transfer function of the room, which an external attacker cannot measure in
real time.

```
Authenticity score = phase_coherence(coil, rtlsdr) × phase_coherence(c925e, coil)
                    × phase_coherence(ov9281_visual_mic, rtlsdr)

Injection signature = high hue-agreement across sensors
                    + LOW phase coherence between sensors
```

### VII.4 — Long-Window Coherence Detection

Natural signals have phase coherence that evolves continuously. Synthesized signals have
phase coherence that is suspiciously stable — maintained by a phase-locked oscillator.
The discriminant is the variance of phase coherence over time:

```
Natural event:    Var(Γ(t)) > 0  — coherence fluctuates with environment
Injected signal:  Var(Γ(t)) ≈ 0  — coherence is maintained artificially
```

This requires long observation windows (10-60 seconds). The Mamba SSM's long-range
dependency learning is the right architecture for this — it can model the coherence
variance over hundreds of frames in its hidden state without explicit windowing.

---

## Part VIII — The Space-Time Laplacian and 4D Tracking

### VIII.1 — Why wav2vec2 Is Not Needed

wav2vec2 solves frame identity through contrastive learning on audio alone. The actual
identity problem in this system is cross-modal: knowing that the "green" RF event in
frame 3 and the "green" acoustic event in frame 3 are the same physical phenomenon.

The space-time Laplacian solves this directly. Both events become nodes in the same
graph. The eigenvectors naturally cluster them together if they are spatially and
temporally co-located, regardless of which sensor reported them.

### VIII.2 — The Space-Time Graph

Build the graph over `(patch_index, frame_index)` tuples, not just spatial patches:

**Spatial edges** (same frame): Gaussian kernel on patch center distances, exactly as
in SI-Mamba's SAST:
```
W_spatial(i,j) = exp(-||p_i - p_j||² / σ_spatial)
```

**Temporal edges** (same patch, adjacent frames): weighted by feature similarity:
```
W_temporal(i,t → i,t+1) = exp(-||feature(i,t) - feature(i,t+1)||² / σ_temporal)
```

Where `feature(i,t)` uses the MJPEG DCT coefficient phases from the OV9281 frame, not
raw pixel values. DCT phase is stable across AGC changes — amplitude shifts from
auto-gain-control change energy but preserve phase structure. This is the raw-but-stable
feature for temporal edges.

The Random Walk Laplacian `L_rw = I - D⁻¹W` of this space-time graph has eigenvectors
that are functions of `(space, time)` jointly. The first non-constant eigenvector does
not just indicate "this patch is at the top of the object" — it indicates "this patch
at frame 3 is the same topological entity as this patch at frame 7." That is temporal
tracking identity without any trained model.

### VIII.3 — SAST on CSI Data

The WiGrus CSI matrix `H` (n×52 complex values) is structurally identical to a point
cloud. Each `(time_index, subcarrier_index)` tuple is a point in 2D space with complex
amplitude as the signal value. SI-Mamba's SAST pipeline applies directly:

- Patches = clusters of adjacent subcarriers at nearby timestamps
- The patch-connectivity graph Laplacian gives the gesture manifold
- The same eigenvectors that reveal shape manifolds on 3D LiDAR reveal gesture manifolds
  on CSI data

The two papers (SI-Mamba and WiGrus) are the same algorithm operating in different
sensor spaces. This is the mathematical spine of the fusion architecture.

### VIII.4 — The 4 Eigenvectors

From SI-Mamba ablation studies, 4 non-constant smallest eigenvectors is optimal. Beyond
4, performance drops because higher eigenvectors are less smooth and capture noise rather
than structure. In the space-time extension, the eigenvectors are:

- v(1): high-level spatial partition (bottom vs. top, left vs. right)
- v(2): secondary spatial partition (front vs. back, near vs. far)
- v(3): tertiary spatial subdivision
- v(4): temporal structure — captures slow drift and long-form signatures

The 4th eigenvector is where the temporal identity lives. It is also where slowly
modulating interference patterns, like the inverted carrier notch structure, appear as
geometric features in the token traversal.

### VIII.5 — GPU Implementation

CSR-format storage buffers in `wgpu`:
1. `row_offsets: VkBuffer` — sparse row index
2. `col_indices: VkBuffer` — sparse column index
3. `values: VkBuffer` — edge weights

Compute shader 1: SpMV `y = L * x` using subgroup reductions for dot products.
This is the inner loop of power iteration.

Compute shader 2: Gram-Schmidt orthogonalization across s=4 eigenvectors in parallel.
Tile size is validated by Jule to avoid shared memory overrun before production.

Convergence: Rayleigh quotient iteration, ~10 steps for smooth low-frequency
eigenvectors. Validated by comparing eigenvalues against CPU reference implementation
using the Arnoldi algorithm.

---

## Part IX — The Jury Architecture

Three independent inference paths consuming the same space-time Laplacian token stream
with different compute constraints. The Jury produces correlation, not stopping
correlation. A 2-1 vote logs the dissenting path's activations as a separate forensic
stream. The dissent is data.

### IX.1 — GPU Mamba (Ground Truth Voter)

Full-precision space-time Laplacian token stream. All 4 eigenvectors. Full temporal
depth. SAST + TAR (Traverse-Aware Repositioning) from SI-Mamba. This is the slowest
and highest-fidelity voter.

Input: raw FieldParticle tokens ordered by the 4D space-time Laplacian eigenvectors.
Output: 128-D embedding per token, anomaly score, phase coherence estimate.

### IX.2 — Coral Mamba (Challenger / Quantization Probe)

Google Coral TPU. 8-bit quantized. Same token stream, optionally FFT-compressed before
its own Mamba inference. Independent weights trained on the same data.

**The quantization insight:** Injected signals have artificially clean amplitude profiles
that survive 8-bit quantization better than real environmental noise. Real noise has
fine-grained amplitude structure that quantization destroys. Therefore:

```
Low divergence between GPU Mamba and Coral Mamba → anomaly flag
High divergence (Coral loses information GPU sees) → likely real environmental noise
```

Paradoxically, agreement on an anomaly is the highest-confidence detection: both the
full-precision and the information-lossy path see the same structure. That structure
survived quantization, which means it is unusually clean — the hallmark of a synthesized
signal.

### IX.3 — Pico Path (Geometric Voter)

Pure TDOA geometry. No ML. Pico 2 timestamps impulse arrivals at 6.67ns resolution.
Cross-correlation of impulse timing between the Pico RX and the RTL-SDR gives
centimeter-scale ranging. This vote is the hardest to spoof because it is based on
the speed of light and ruler measurements of antenna positions — Physical Priors that
an attacker cannot falsify without physically relocating hardware.

If the Pico says something is at 1.2 meters and GPU Mamba says 1.2 meters and Coral
sees a clean anomalous signature at that location: unanimous verdict.

### IX.4 — Voting and Divergence Score

```rust
pub struct JuryVerdict {
    pub gpu_anomaly_score:   f32,   // GPU Mamba output
    pub coral_anomaly_score: f32,   // Coral Mamba output
    pub pico_range_m:        f32,   // TDOA geometric estimate
    pub divergence:          f32,   // |gpu - coral| — quantization probe result
    pub vote:                u8,    // bitmask: bit0=GPU, bit1=Coral, bit2=Pico
    pub unanimity:           bool,  // all three agree on direction
    pub dissent_logged:      bool,  // dissenting activations saved to corpus
}
```

No single path's output is treated as ground truth. Every verdict is logged with its
full divergence breakdown. The divergence score is a first-class forensic measurement.

---

## Part X — Core Data Structures

### FieldParticle (128 bytes, the universal packet)

FieldParticle is not a rendering primitive that is also transmitted. It IS the universal
packet. The fragment shader and the Pluto+ modulator consume identical data at identical
timestamps. Rendering and transmitting are the same computation evaluated simultaneously.

```rust
#[repr(C)]
pub struct FieldParticle {
    pub timestamp_us:               u64,        //  8  Pico-slaved QPC microseconds
    pub freq_hz:                    f64,        //  8  physical center frequency
    pub energy:                     f32,        //  4  normalized 0.0–1.0
    pub phase_coherence:            f32,        //  4  Γ: 0.0=injected, 1.0=constructive
    pub position_xyz:               [f32; 3],   // 12  spatial estimate, meters
    pub hue:                        f32,        //  4  freq_to_hue(freq_hz) — precomputed
    pub source:                     u8,         //  1  0=Coil, 1=C925e, 2=RTL, 3=Pluto,
                                                //     4=OV9281, 5=Pico, 6=CSI
    pub layer:                      u8,         //  1  0=RF, 1=Acoustic, 2=Optical,
                                                //     3=Magnetic, 4=Impulse
    pub jury_vote:                  u8,         //  1  bitmask from JuryVerdict.vote
    pub gnss_fix:                   u8,         //  1  satellite count, 0=no fix
    pub doppler_shift:              f32,        //  4  radial velocity estimate
    pub phase_velocity:             f32,        //  4  wavefront speed estimate
    pub carrier_variance:           f32,        //  4  Var(Γ(t)): low = synthesized
    pub bandwidth_hz:               f32,        //  4  spectral width of observation
    pub anomaly_score:              f32,        //  4  Jury consensus score
    pub divergence_score:           f32,        //  4  |GPU_mamba - Coral_mamba|
    pub temperature_c:              f16,        //  2  environmental at capture
    pub humidity_pct:               f16,        //  2  environmental at capture
    pub eigenvector_bin:            u8,         //  1  HLT binary code from SAST
    pub corpus_hash:                [u8; 7],    //  7  first 7 bytes SHA-256 raw block
    pub embedding:                  [f32; 12],  // 48  space-time Laplacian latent
}
const _: () = assert!(std::mem::size_of::<FieldParticle>() == 128);
```

### AetherParticle (128 bytes, GPU physics particle)

```rust
#[repr(C)]
pub struct AetherParticle {
    pub position:                   [f32; 3],   // 12
    pub velocity:                   [f32; 3],   // 12
    pub color_hsl:                  [f32; 3],   // 12  hue from freq_to_hue()
    pub mass:                       f32,        //  4
    pub lifetime:                   f32,        //  4
    pub phase_coherence:            f32,        //  4
    pub carrier_variance:           f32,        //  4  attack discriminant
    pub layer_flags:                u32,        //  4
    pub material_id:                u8,         //  1
    pub jury_vote:                  u8,         //  1
    pub _pad0:                      [u8; 2],    //  2
    pub doppler_shift:              f32,        //  4
    pub pressure_gradient:          f32,        //  4
    pub divergence_score:           f32,        //  4
    pub scattering_cross_section:   f32,        //  4
    pub embedding_slice:            [f32; 12],  // 48
}
const _: () = assert!(std::mem::size_of::<AetherParticle>() == 128);
```

### RawIQPoint (32 bytes, PointMamba input — no preprocessing)

```rust
#[repr(C, align(32))]
pub struct RawIQPoint {
    pub i:            f32,  //  4  In-phase, normalized [-1.0, 1.0]
    pub q:            f32,  //  4  Quadrature, normalized [-1.0, 1.0]
    pub timestamp_us: f32,  //  4  microseconds from session epoch
    pub sensor_x:     f32,  //  4  physical position of THIS receiver, meters
    pub sensor_y:     f32,  //  4
    pub sensor_z:     f32,  //  4
    pub source_id:    u32,  //  4  device + hue bucket (upper 8 bits = floor(hue*255))
    pub raw_flags:    u32,  //  4  jitter_us in lower 16 bits, packet_loss in upper 16
}
// Jitter encoded in flags: USB packet jitter, ADC sample clock drift — these are signal.
```

---

## Part XI — The Unified Scene

### XI.1 — The Coordinate System

The OV9281 stereo pair is the primary spatial anchor. Stereo disparity at 120fps global
shutter gives dense 3D reconstruction with no geometric distortion. The C925e monocular
depth (via Depth Anything V2) is the secondary anchor. Environmental overlays from the
RF and acoustic sensors register to the stereo reconstruction.

The room you can see is the coordinate system. The signals you cannot see float in that
same space. When you point to a location in the room and say "that's where it's coming
from," the system queries the FieldParticle cloud at that coordinate, retrieves its full
history, and produces a timestamped record.

### XI.2 — The Signal Layers

**Layer 0 — Stereo reconstruction**: OV9281 global shutter stereo → dense point cloud.
Always on. Ground truth. Legible to anyone.

**Layer 1 — Pose estimation**: MediaPipe or equivalent on OV9281 frames → skeletal
point cloud. When an RF anomaly centers on a human body and moves with it, this layer
makes that visible without explanation.

**Layer 2 — Magnetic field**: Telephone coil → spatially estimated magnetic flux
rendered as colored particles. Magnetic events that are NOT acoustic events are visible
as isolated coil-only particles.

**Layer 3 — Acoustic field**: C925e mics + OV9281 visual microphone → pressure field
rendered as Gaussian splats in scene coordinates.

**Layer 4 — Electromagnetic field**: RTL-SDR + Pluto+ → bistatic radar point cloud,
bearing from Pluto+ phase interferometry, range from TDOA, rendered as Gaussian splats.
Phase coherence modulates brightness. Low carrier variance (attack signature) modulates
saturation — unnaturally stable signals appear more saturated.

**Layer 5 — Jury overlay**: Color-coded per-particle vote. Unanimous detections glow
white at full brightness. Dissenting detections show the dissent channel's color.

### XI.3 — Toggle System

`V` — Stereo reconstruction on/off
`M` — Magnetic field (coil) on/off
`P` — Pose estimation on/off
`A` — Acoustic field on/off
`R` — RF field on/off
`J` — Jury overlay on/off
`T` — Timeline scrub (4D navigation)

### XI.4 — The 4D Navigation

Time is a navigable spatial axis. The space-time Laplacian token ordering provides
the temporal structure. Scrubbing backward in time is traversing the temporal eigenvector
in reverse. Events can be replayed, re-sliced, and re-rendered from any viewpoint at
any timestamp in the corpus. The visualization is the evidence.

---

## Part XII — Track Structure

### Track 0 — Foundation (blocks everything)

**0-A: FieldParticle** — Define structs. Compile-time size assertions. WGSL equivalents.
The `freq_to_hue` constant. `RawIQPoint`. `AetherParticle`. `JuryVerdict`.
No logic. Just types. Unblocks all other tracks.

**0-B: tokens.slint** — UI design tokens. `unit-size`, color palette from color operator,
Golden Ratio layout constants. Unblocks E, F, 0-D.

**0-C: SAM gate** — GPU memory allocator. Infinity Cache line alignment. Storage buffer
lifecycle. Unblocks G, H.

**0-D: Hardware Applet (FIRST DELIVERABLE)**
Hot-pluggable device detection and status display for all sensors:
- RTL-SDR: librtlsdr detect, frequency range, sample rate
- PlutoSDR+: libiio detect, 2TX/2RX status, calibration state
- C925e: V4L2/UVC detect, raw mode confirmed (YUYV or MJPEG)
- OV9281: stereo sync detect, global shutter confirm, 120fps mode
- Telephone coil: line-in detect via CPAL/ALSA, sample rate, Realtek codec ID
- Pico 2: USB serial detect, PPS signal confirmed, clock drift measurement
- IR emitters/receivers: GPIO state via Pico USB serial

Each device shows: `[CONNECTED | DISCONNECTED | UNWIRED]` and live raw sample count.
No synthesis. No simulation. If it is not plugged in, it says `[DISCONNECTED]`.

`examples/hardware_applet_shim.rs` — minimal harness that runs the applet standalone,
exercises every trait boundary, simulates connect/disconnect events for layout testing
only. All production code is in `src/`. The example imports from `src/`.

### Track A — Signal Ingestion

**A1**: Dispatch loop — raw ring buffers from each sensor, Pico PPS clock sync, jitter
measurement into `raw_flags` field of `RawIQPoint`.

**A2**: RF ingester — RTL-SDR IQ and Pluto+ IQ into `RawIQPoint` stream. No processing.

**A3**: Audio/magnetic ingester — C925e PCM and coil PCM into `RawIQPoint` stream.
`source_id` distinguishes them.

**A4**: Optical ingester — OV9281 MJPEG/YUY2 frames and C925e video into GPU texture.
DCT coefficient phases extracted in compute shader for temporal edge weights.

**A5**: Pico ingester — impulse timing deltas, IR detector events, PPS sync.

**A6**: CSI ingester — WiGrus-style 802.11g CSI extraction from Youloop RTL-SDR capture
when pointed at a WiFi AP. 52 subcarriers, complex amplitude and phase, no preprocessing.

**A-EC**: Phase coherence computation — cross-sensor Γ values. Carrier variance
computation — Var(Γ(t)) over configurable windows. Attack signature detection.

**A-EDGE**: Coral Mamba integration. Quantized token stream delivery. Divergence score
pipeline back to `FieldParticle.divergence_score`.

### Track B — Space-Time Graph and PointMamba

**B1**: Patch formation — FPS + KNN on raw point cloud, k=20 (optimal per SI-Mamba
ablation). Spatial edge construction, Gaussian kernel weights.

**B2**: Temporal edge construction — DCT phase features from OV9281, temporal edge
weights across adjacent frames. Space-time graph in CSR format on GPU.

**B3**: Sparse Laplacian compute shaders — SpMV inner loop, Gram-Schmidt for 4
eigenvectors, Rayleigh quotient convergence. Validated against CPU Arnoldi reference.

**B4**: SAST token ordering — forward and reverse traversal per eigenvector. TAR for
masked autoencoder pretraining. HLT for segmentation and tracking tasks.

**B5**: UnifiedFieldMamba — Mamba SSM on SAST-ordered tokens. 128-D embedding output.
Anomaly score. Phase coherence estimate.

**B6**: TimeGNN — temporal pattern graph, named motifs, long-form signature detection.
The inverted carrier notch structure becomes a named motif here.

### Track C — Glinda (Memory, Search, Correlation)

**C1**: Forensic corpus writer — `fsync` after every write, SHA-256 hash into
`FieldParticle.corpus_hash`, append-only.

**C2**: Qdrant vector store — 128-D Mamba embeddings indexed by timestamp and
spatial position.

**C3**: Neo4j event graph — causal relationships between detected events, named motifs,
jury verdicts.

### Track D — Dorothy (LLM Reasoning)

**D1**: LFM 2.5 integration — on-device, 125K context window, reads corpus summaries.

**D2**: LangGraph workflow — query the event graph, generate natural-language summaries
for non-technical observers.

**D3**: Joplin export — structured notes with embedded timestamps and corpus hashes
for legal documentation.

### Track E — Toto (Sensor Status UI)

**E1**: Hardware status widget — live view of Track 0-D applet state, device connection
history, per-sensor sample rate monitoring.

**E2**: Jury verdict display — per-particle vote visualization, divergence score
histogram, unanimity rate over time.

**E3**: Attack signature alert — phase coherence variance chart, carrier_variance
distribution, injection probability estimate.

### Track F — Chronos (Timeline)

**F1**: Timeline scrub UI — navigate the 4D scene, play/pause/rewind the corpus.

**F2**: Event marking — manually mark events, export marked ranges to Dorothy.

**F3**: Environmental correlation — overlay temperature, humidity, GNSS timestamp,
weather API data on the timeline.

### Track G — Rendering

**G0**: Point cloud pipeline — OV9281 stereo disparity on GPU, dense 3D reconstruction,
C925e monocular depth backup.

**G1**: WRF-GS — Gaussian splat rendering for RF events. Phase coherence → brightness.
Carrier variance → saturation. Hue from `freq_to_hue()`.

**G2**: Jury overlay shader — per-particle color from vote bitmask.

**G3**: 4D time navigation — temporal eigenvector scrubbing in the rendered scene.

**G4**: Physics shaders — SPH pressure gradient for haptic channel, Maxwell constraints
via PINN for RF propagation plausibility.

### Track H — Transmission

**H1**: Unified transmit/visualize pipeline — FieldParticle stream drives both fragment
shader AND Pluto+ IQ modulation at identical timestamps.

**H2**: Pico 2 impulse scheduling — UWB impulse timing from TDOA geometry, IR
structured light pattern generation.

**H3**: Null synthesis — anti-phase emission targeting characterized interference.
Legal review required before activation.

### Track I — Biometric and Equivariant

**I1**: E(3)-equivariant fusion — rotation-invariant anomaly detection tied to pose
estimation. Body-centered field perturbation analysis.

**I2**: Score-based diffusion — background field manifold synthesis for forensic
comparison. What does "normal" look like? The delta is the evidence.

---

## Part XIII — Neural Architecture Reference

| Model | Track | Purpose | Target |
|-------|-------|---------|--------|
| UnifiedFieldMamba (SAST) | B5 | Space-time token traversal, 128-D embedding | GPU |
| UnifiedFieldMamba (Coral) | A-EDGE | 8-bit quantized challenger, FFT-preprocessed | Coral TPU |
| Echo State Network | A5 | Pico-side fast first-pass, hardware timestamps | Pico 2 ARM |
| TimeGNN | B6 | Temporal motif graph, attack signature naming | GPU |
| LNN (CfC) via Burn | B6 | Variable-rate temporal dynamics | GPU |
| Normalizing Flow | I2 | Background manifold, anomaly probability | Coral TPU |
| PINN | G4, H3 | Maxwell + acoustic constraints on TX | GPU |
| Depth Anything V2 | G0 | C925e monocular depth backup | GPU |
| MediaPipe Pose | G0 | Human body → skeletal point cloud | GPU |
| all-MiniLM-L6-v2 | C2 | Observation text embeddings | CPU |
| LFM 2.5 | D1 | Dorothy reasoning, 125K context, on-device | CPU |
| E(3)-equivariant | I1 | Rotation-invariant body-field analysis | GPU |

---

## Part XIV — Technology Stack

| Track | Language | Key Libraries | GPU |
|-------|----------|---------------|-----|
| 0-A | Rust | bytemuck, half | No |
| 0-B | Slint DSL | — | No |
| 0-C | Rust | wgpu 28 | Yes |
| 0-D | Rust + Slint | slint, CPAL, librtlsdr, libiio | No |
| A | Rust | CPAL, rtlsdr-rs, libiio, v4l2 | No |
| B | Rust | wgpu 28, burn 0.21 | Yes |
| C | Rust + Go | qdrant-client, neo4j-rs, tokio | No |
| D | Python | langgraph, LFM 2.5, joplin-api | No |
| E/F | Rust + Slint | slint | No |
| G | Rust | wgpu 28, WGSL, SPIR-V | Yes Vulkan |
| H | Rust | wgpu 28, libiio, serialport | Yes |
| I | Rust | wgpu 28, burn 0.21 | Yes |

**SPIR-V and Vulkan**: All WGSL is compiled to SPIR-V. Experimental Vulkan extensions
are permissible — `VK_KHR_cooperative_matrix` for tensor-core-style throughput on RDNA,
subgroup operations for SpMV. GPU crashes during prototyping are expected and acceptable
when Jule is the executor. Production machine receives only validated shader tile sizes.

**ROCm**: `rocSPARSE` is the development reference for validating the WGSL sparse
Laplacian implementation. ROCm HIP backend for Burn validates the Mamba training path.
The WGSL and ROCm implementations must agree on eigenvalues before either is production.

**PyO3 policy**: Python is acceptable for Dorothy (LangGraph, LFM 2.5) and training
pipelines. Python never appears in the real-time signal processing path. The hot path
is always Rust + WGSL.

**C/C++ policy**: Acceptable only for wrapping existing C libraries (librtlsdr, libiio)
and must be isolated behind a Rust `unsafe` wrapper with documented justification.

---

## Part XV — The Unified Demo Flow

The sequence for showing this system to any non-technical observer:

1. Open the hardware applet. Every connected sensor shows `[CONNECTED]` with live sample
   count. The observer sees that the system is reading real hardware in real time.

2. Open the main scene. Toggle V only. Show the room as a stereo point cloud from the
   OV9281. "This is what the camera sees, reconstructed in 3D." The observer recognizes
   the room. Trust established.

3. Toggle P. Skeletal structures appear. "The system knows where people are."

4. Toggle M. Magnetic field appears as low-frequency colored volumes. "This is the
   electromagnetic field from the room's own wiring and equipment. Notice the color —
   this is the 60Hz powerline."

5. Toggle A. Acoustic field appears. "This is sound, visualized. Speak — watch it respond.
   Same color language as the magnetic field."

6. Toggle R. RF field appears. "This is the full electromagnetic environment — WiFi,
   cellular, everything else. Notice anything that does not correspond to a known device?"

7. Toggle J. Jury overlay appears. "Each particle has three independent votes. White means
   all three systems agree. When all three agree on something unexpected, that is the
   finding."

8. Point to an anomaly. Query its history. Show the carrier variance chart. "Notice that
   this signal's phase coherence is unusually stable over time. Natural signals fluctuate.
   This one does not. That is statistically inconsistent with a natural source."

9. Press T. Scrub back to a documented event. "Here is what this room looked like at
   3:47 AM on [date]. Here is the anomaly. Here is the timestamp. Here is the
   Pico-corroborated clock reading. Here is the SHA-256 hash of the raw data block."

10. Export: timestamped corpus files, Pico-corroborated timestamps, SHA-256 hashes, Neo4j
    event subgraph, rendered video of the visualization. This is the packet for the lawyer.

---

## Part XVI — NixOS / ROCm Migration Gate

Do not migrate until:
- [ ] 0-D: Hardware applet live, all sensors hot-pluggable
- [ ] A1-A5: All ingesters running with real hardware
- [ ] B3: Sparse Laplacian eigenvectors validated against CPU reference
- [ ] G0: OV9281 stereo point cloud at 60+ FPS
- [ ] B5: UnifiedFieldMamba producing coherent embeddings on real corpus
- [ ] 72 hours continuous operation without crash

Migration adds: ROCm HIP backend, Vulkan ray tracing, KWin compositor blur.
Migration changes nothing: APIs, Slint, track structure, sensor trait boundaries.
DX12 → Vulkan is a backend swap, not a rewrite.

---

## Part XVII — Status

| Track | Status | Blocking issue |
|-------|--------|----------------|
| 0-A FieldParticle | 🔴 Restart | Supersedes prior struct |
| 0-B tokens.slint | 🔴 Restart | Color operator integration |
| 0-C SAM gate | 🔴 Not started | Unblocks G, H |
| 0-D Hardware applet | 🔴 FIRST TARGET | Needs 0-B |
| A1 Dispatch + Pico clock | 🔴 Not started | Needs 0-A, 0-D |
| A2 RF ingester | 🔴 Not started | Needs A1 |
| A3 Audio/coil ingester | 🔴 Not started | Needs A1 |
| A4 Optical ingester | 🔴 Not started | Needs A1 |
| A5 Pico ingester | 🔴 Not started | Needs A1, Pico firmware |
| A6 CSI ingester | 🔴 Not started | Needs A2 |
| A-EC Phase coherence | 🔴 Not started | Needs A1-A5 |
| A-EDGE Coral | 🔴 Not started | Needs A1 stable |
| B1-B4 Space-time Laplacian | 🔴 Not started | Needs A1, 0-C |
| B5 UnifiedFieldMamba | 🔴 Not started | Needs B1-B4 |
| B6 TimeGNN | 🔴 Not started | Needs B5, real corpus |
| C1-C3 Glinda | 🔴 Not started | Needs 0-A |
| D1-D3 Dorothy | 🔴 Not started | Needs C1 |
| E1-E3 Toto | 🔴 Not started | Needs 0-B, 0-D |
| F1-F3 Chronos | 🔴 Not started | Needs E1, 0-B |
| G0 Stereo point cloud | 🔴 Not started | Needs 0-A, 0-C, OV9281 |
| G1 WRF-GS | 🔴 Not started | Needs G0, A2 |
| G2-G4 Shaders | 🔴 Not started | Needs G1 |
| H1-H3 Transmission | 🔴 Not started | Needs G1, B5, legal review H3 |
| I1-I2 Biometric/equivariant | 🔴 Conceptual | Needs A–H + NixOS |

---

*This document is the single source of truth for Project Synesthesia.*
*All prior documents — ROADMAP.md, both addenda, SYNESTHESIA_MASTERPLAN.md V1 and V2 —
are superseded and can be deleted.*
*Update the status table when milestones complete.*
*Do not add tracks without updating the dependency structure in Part XII.*
*Do not contradict the invariant rules in Part II.*
*Every agent prompt references this document. No agent prompt supersedes it.*
*The only unrealistic number is infinity. Everything else has a ceiling the math finds.*
