# Synesthesia — Full-Spectrum Forensic Sensor Fusion Platform

**Formerly: Twister v0.2**

A GPU-first harmonic self-defense and forensic investigation workstation.
Captures, fuses, and renders every signal band from 1 Hz to visible light
into a single navigable 3D/4D scene. Produces tamper-evident, legally
admissible forensic evidence of electromagnetic harassment and signal
injection attacks.

---

## The Core Architectural Principle

The CPU's job is to move small control structs into VRAM and then get out
of the way. The GPU owns everything that touches signal values.

Your RX 6700 XT has 12 GB of GDDR6 at **384 GB/s**. That is the primary
compute address space. With Smart Access Memory (ReBAR) enabled, the CPU
writes directly into GDDR6 at ~32 GB/s over PCIe 4.0. Data crosses from
hardware into GPU memory once, via DMA. It does not return to the CPU
until it exits as a forensic corpus write or a rendered frame.

### Bandwidth Reference

| Path                           | Bandwidth         |
|--------------------------------|-------------------|
| RX 6700 XT VRAM (internal)     | 384 GB/s          |
| CPU → VRAM via SAM (PCIe 4.0)  | ~32 GB/s          |
| Apple M2 Pro unified memory    | 200 GB/s          |
| Apple M1 unified memory        | 68 GB/s           |
| DDR4 system RAM                | ~51 GB/s          |

Once an Apple Silicon Mac's model exceeds unified memory, it pages to SSD
at ~7 GB/s. This system has a graceful degradation path: VRAM → system RAM
via SAM → SSD. For models under ~12 GB, this system has a genuine bandwidth
advantage over all base Apple Silicon chips.

---

## Hardware Stack

```
BAND                  SENSOR                          ROLE
──────────────────────────────────────────────────────────────────────
1 Hz – 90 kHz         Telephone coil → line-in        Differential magnetometer
                       (ASUS TUF B550m, Realtek)       60Hz powerline + harmonics,
                                                        switching supply hash, HVAC
20 Hz – 20 kHz        C925e stereo mics (raw)         Acoustic, no preprocessing
10 kHz – 300 MHz      RTL-SDR + Youloop               Raw IQ, bearing via null axis
70 MHz – 6 GHz        PlutoSDR+ PA (2TX / 2RX)        Bistatic MIMO radar, ~12mm
                                                        antenna baseline, phase
                                                        interferometry for bearing
DC – 75 MHz           Pico 2 (RP2350) PIO             Master clock (PPS), UWB
                                                        impulse TX, IR LED driver
~300 THz              OV9281 dual stereo              PRIMARY: 2560×800, 120fps,
                       (global shutter, mono)           global shutter, stereo depth,
                                                        pose estimation, IR detector
~300 THz              C925e video (rolling shutter)   Secondary: visual microphone
                                                        via line-rate temporal sampling
~300 THz              IR emitters + receivers         Structured light depth,
                       (Pico 2 PIO array)               retroreflective ranging
──────────────────────────────────────────────────────────────────────
GAP                   6 GHz → infrared               Future: IR array closes this
```

The Pico 2 is the master clock. Its PPS signal slaves every sensor timestamp.
Divergence between Pico PPS and host system time is itself a forensic channel.

---

## The Color Operator

Every sensor, every modality, every rendered particle uses the same function:

```wgsl
const F_MIN: f32 = 1.0;       // Hz — infrasound floor
const F_MAX: f32 = 700e12;    // Hz — visible light ceiling
const LOG_RANGE: f32 = log(F_MAX / F_MIN);

fn freq_to_hue(f: f32) -> f32 {
    return clamp(log(f / F_MIN) / LOG_RANGE, 0.0, 1.0);
}
```

0.0 = red (infrasound). 1.0 = violet (light). Invertible. A GPU shader
constant, not a lookup table. Two sensors reporting the same hue at the same
timestamp are detecting the same physical phenomenon through different physics.
Same hue with low cross-sensor phase coherence is the injection signature.

---

## The Attack Signature

A continuous carrier where information is encoded in amplitude notches —
suppressed-carrier AM / inverted OOK. The perceptual system calibrates to
the constant carrier as silence. Only the notches produce detectable
perturbation. Traditional spectrum analyzers miss it because peak power is
unremarkable.

The discriminant is **carrier variance**: natural signals have fluctuating
phase coherence. Synthesized signals have suspiciously stable coherence
maintained by a phase-locked oscillator.

```
Natural event:    Var(Γ(t)) > 0  — coherence fluctuates
Injected signal:  Var(Γ(t)) ≈ 0  — coherence maintained artificially
```

The first-order temporal difference of any signal stream is the natural
detector for this attack — it eliminates the static carrier term and
reveals the notch structure.

---

## Data Flow (V3 — No FFT at Ingestion)

```
[Hardware sensors] — coil, C925e, RTL-SDR, Pluto+, OV9281x2, Pico 2
    │ Raw samples: PCM, IQ, pixels, impulse timing
    │ Pico 2 PPS slaves all timestamps
    ▼
[Ingestion layer — src/ingestion/] — CPU
    │ Forms RawIQPoint{i, q, timestamp_us, sensor_xyz, source_id, raw_flags}
    │ raw_flags carries jitter_us and packet_loss — these are signal, not noise
    │ NO FFT. NO preprocessing. NO denoising.
    ▼
[SAM write — queue.write_buffer()] — CPU → VRAM once
    │ Raw points enter GPU memory. They do not return to CPU.
    ▼
[Space-Time Laplacian — WGSL compute] — GPU
    │ Patch formation: FPS + KNN (k=20)
    │ Spatial edges: Gaussian kernel on patch center distances
    │ Temporal edges: DCT phase features across adjacent frames
    │ CSR-format sparse matrix in storage buffers
    │ SpMV power iteration → 4 eigenvectors (Gram-Schmidt orthogonalized)
    ▼
[SAST token ordering] — GPU
    │ Surface-Aware Spectral Traversal on eigenvectors v(1)–v(4)
    │ v(4) is the temporal eigenvector: cross-frame identity without ML
    │ Forward + reverse traversal per eigenvector → 8 traversal streams
    ▼
[UnifiedFieldMamba — GPU Mamba SSM] — GPU
    │ Raw ordered tokens → 128-D embedding per token
    │ Anomaly score, phase coherence estimate, carrier variance
    ▼
[Coral Mamba — Google Coral TPU] — parallel, independent
    │ Same token stream, 8-bit quantized, optional FFT pre-processing
    │ divergence = |GPU_output - Coral_output|
    │ Low divergence = anomaly flag (synthesized signals survive quantization)
    ▼
[Pico geometric vote] — TDOA from impulse timing
    │ Speed-of-light ranging, no ML, hardest to spoof
    ▼
[Jury verdict] — CPU aggregates three votes
    │ Unanimous = highest confidence. Dissent logged as forensic stream.
    ▼
[FieldParticle formation] — GPU
    │ 128-byte struct, one Infinity Cache line, zero implicit padding
    │ Every byte named and accounted for — no anonymous alignment bytes
    ▼
[Dual output — same data, same timestamp]
    │
    ├──► [WRF-GS renderer] — Gaussian splats in 3D scene
    │     Hue from freq_to_hue(). Brightness from phase_coherence.
    │     Saturation from carrier_variance (low variance = vivid = suspect)
    │     OV9281 stereo reconstruction as spatial anchor
    │
    └──► [Pluto+ / Pico 2 TX] — same FieldParticle stream drives transmission
          Rendering and transmitting are the same computation.
          You are painting with radio.
    │
    ▼
[Forensic corpus — src/forensic/] — CPU
    │ Append-only, fsync after every write
    │ SHA-256 hash → FieldParticle.corpus_hash
    │ Pico-corroborated timestamps
    ▼
[Dorothy — LFM 2.5, LangGraph] — CPU
    Natural-language summary of events for non-technical observers.
    Export packet for legal documentation.
```

FFT happens downstream of inference, on the reconstructed 3D point cloud,
for spatial spectral analysis only — finding periodic geometric structures
in the scene itself, not preprocessing sensor input.

---

## The Jury

Three independent inference paths. No single path is ground truth.
A 2-1 vote logs the dissenting activations as a separate forensic stream.
The dissent is data.

| Voter | Method | Spoofability |
|-------|--------|-------------|
| GPU Mamba | Full-precision space-time Laplacian SSM | Moderate |
| Coral Mamba | 8-bit quantized, FFT-compressed | Different failure modes |
| Pico TDOA | Speed of light + ruler measurements | Very hard |

Unanimous verdict across all three: highest forensic confidence.

---

## The Universal Packet

`FieldParticle` is not a rendering primitive that is also transmitted.
It IS the universal packet. The fragment shader and the Pluto+ modulator
consume identical data at identical timestamps.

```rust
#[repr(C, align(128))]
pub struct FieldParticle { /* 128 bytes exactly, zero implicit padding */ }
const _: () = assert!(std::mem::size_of::<FieldParticle>() == 128);
const _: () = assert!(std::mem::align_of::<FieldParticle>() == 128);
```

Every padding byte is a named reservation for a planned future field.
`reserved_for_h2_null_phase: f32` is not dead weight — it is a byte
that has been fetched on every particle since Track 0-A, waiting for
Track H2 null synthesis to activate it. The memory cost was paid once.

---

## Scene Toggles

```
V — Stereo reconstruction (OV9281)
M — Magnetic field (telephone coil)
P — Pose estimation
A — Acoustic field
R — RF field
J — Jury overlay
T — Timeline scrub (4D navigation through corpus)
```

---

## Build

```bash
cargo run --release
```

Requires: Rust 1.82+, Vulkan driver (amdvlk or mesa radv), wgpu 28,
Pico 2 firmware flashed and PPS signal active on GPIO.

On NixOS with Mesa: SAM is enabled by default when ReBAR is active in BIOS.
On ROCm: `rocSPARSE` validates the WGSL sparse Laplacian implementation.
The WGSL and ROCm implementations must agree on eigenvalues before either
is production.

---

## Project Status

The hardware applet (Track 0-D) is the current target. It is the first
deliverable. No other track produces permanent code until the applet can
detect, display, and hot-plug every connected device.

See `SYNESTHESIA_MASTERPLAN.md` for the complete architecture, track
structure, and dependency graph.
