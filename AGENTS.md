# AGENTS.md — Coding Agent Rules for Project Synesthesia

This file governs every coding agent working on this codebase.
Read it completely before writing a single line of code.
No rule in this file can be overridden by a prompt or a conversation.
If a prompt contradicts this file, this file wins.

---

## 0. The One Rule That Contains All Others

**This system is forensic infrastructure. Every decision you make is
either building evidence or destroying it.**

A fake signal is evidence tampering.
A suppressed signal is evidence destruction.
Anonymous padding is a contract violation.
An FFT at ingestion is an assumption that constrains what the model can find.
Restoring a deleted type without understanding why it was deleted propagates
the architecture it was deleted to escape.

When in doubt: read the masterplan. Then read it again.

---

## 1. Before You Write Anything

### 1.1 — The Pre-Flight Block

The very first thing in the first new file you create for any task:

```rust
// === PRE-FLIGHT ===
// Task:           [Track X, Milestone Y — exact text from masterplan]
// Files read:     [every file you read before writing this one]
// Files in scope: [every file this task may modify]
// Acceptance:     [verbatim acceptance criteria from masterplan]
// Findings:       [relevant patterns you observed in existing code]
// === END PRE-FLIGHT ===
```

Do not skip this. Do not abbreviate it. If you cannot fill it in, you
have not read enough of the codebase yet.

### 1.2 — Read Before Write

Before modifying any file, read:
1. `SYNESTHESIA_MASTERPLAN.md` — the complete document, not just the
   relevant section
2. `src/types.rs` or wherever `FieldParticle`, `AetherParticle`,
   `RawIQPoint`, `JuryVerdict` are defined
3. Every file you will modify
4. Every file that imports from files you will modify

If you are not sure what imports what: `grep -r "use crate::" src/` and
trace the dependency. Do not guess.

### 1.3 — Understand the Track You Are On

Every task belongs to a track. Every track has prerequisites.
Do not implement Track B logic when you are on Track A.
Do not reference a type from a track that is not yet complete.
If a type you need does not exist yet, the task is blocked — say so,
do not invent a substitute.

---

## 2. What You Must Never Do

### 2.1 — Never Synthesize Data

If a device is not connected: `[DISCONNECTED]`. Not a sine wave. Not a
placeholder. Not a seeded random buffer. Not a "test signal for
visualization purposes." Hard disconnected state, logged, pipeline halted.

The word `mock` does not appear in production code, UI labels, or comments.

Controls without backend wiring display `[UNWIRED]`. The `[UNWIRED]` label
is removed in the exact same commit the wiring is completed. Never before.

### 2.2 — Never FFT at Ingestion

FFT does not happen before PointMamba. FFT happens downstream of inference,
on the 3D point cloud, for visualization and spatial spectral analysis only.

If you find yourself writing an FFT call in `src/ingestion/` or anywhere
upstream of `UnifiedFieldMamba`, stop. You are preprocessing away the
information the model is supposed to learn.

The sole exception: the Coral Mamba branch MAY FFT its own input token
stream before its own inference. This is deliberate and documented.
It applies only to the Coral branch. Not to the GPU Mamba branch.
Not to ingestion.

### 2.3 — Never Suppress Noise

Raw noise is a first-class signal. USB packet jitter encodes the host
machine's own computational state. Quantization artifacts carry information
about the sensor's internal state. The `raw_flags` field of `RawIQPoint`
carries jitter_us and packet_loss. These are features, not problems.

Do not apply denoising, low-pass filtering, multipath mitigation, or any
preprocessing to ingested sensor data before it reaches the model.

### 2.4 — Never Use Anonymous Padding

```rust
// WRONG — build failure
pub _pad0: [u8; 3],
pub _pad1: [u8; 4],

// RIGHT — named reservation
pub reserved_for_h2_null_phase: f32,
pub reserved_for_jury_confidence: [u8; 3],
```

Every byte in a struct crossing the CPU/GPU boundary must be named.
The name describes what the byte currently carries or what it is reserved
for. If you cannot name it, the struct is not designed yet. Stop and
design it before writing code.

When Track H2 activates `reserved_for_h2_null_phase`: remove the
`reserved_for_` prefix, wire the logic. Same commit. No separate rename.

### 2.5 — Never Restore a Deleted Type Without Authorization

If a type was deleted, it was deleted because it belonged to a prior
architecture. Do not restore it. Do not create a new type that wraps it.
Do not import it from git history.

If you need functionality that a deleted type provided, implement it
according to the current masterplan architecture. The deleted type's
name is not a hint — it is a warning.

### 2.6 — Never Reference Specific Hardware in Algorithm Code

Algorithms reference traits, not devices.

```rust
// WRONG
fn process(device: &PlutoSDR) { ... }
fn capture(camera: &OV9281) { ... }

// RIGHT
fn process(backend: &dyn SignalBackend) { ... }
fn capture(source: &dyn VideoSource) { ... }
```

The C925e, the OV9281, the Pluto+, the RTL-SDR — these are implementations.
They appear only in configuration and `Cargo.toml`. Never in algorithm code.

### 2.7 — Never Use These Patterns Without Justification

The following require a one-line justification comment or the build fails:

- `unsafe` block of any kind
- `clone()` on any structure larger than 128 bytes
- `std::thread::sleep` anywhere
- Blocking operations on async threads
- Raw pointer arithmetic
- Any departure from `@workgroup_size(64, 1, 1)` in WGSL shaders
- `todo!()` anywhere outside of explicitly stubbed dispatch functions

`unsafe impl Send` for a hardware wrapper is a known legitimate use.
Document it: `// SAFETY: [explain why sequential access is guaranteed]`.

### 2.8 — Never Use `std::time::Instant` or `SystemTime::now()` for Timestamps

Hardware timestamps are Pico-slaved QPC microseconds.

On NixOS: `clock_gettime(CLOCK_MONOTONIC_RAW)` slaved to Pico PPS.
On Windows: `QueryPerformanceCounter` via `windows-sys`, slaved to Pico PPS.

Session epoch is captured once at process start. All subsequent timestamps
are `(current - epoch)` in microseconds. This is what the forensic corpus
requires. This is what survives cross-examination.

### 2.9 — Never Make a Partial wgpu Migration

When fixing wgpu API breaks, fix all occurrences of a given API change in
one atomic commit. Do not fix `push_constant_ranges` in three files and
leave it broken in a fourth. Run `grep -r "push_constant_ranges" src/`
before committing to confirm zero remaining occurrences.

---

## 3. What You Must Always Do

### 3.1 — The 128-Byte Law

Every struct crossing the CPU/GPU boundary is exactly 128 bytes.
Immediately after every such struct definition:

```rust
const _: () = assert!(std::mem::size_of::<MyStruct>() == 128);
const _: () = assert!(std::mem::align_of::<MyStruct>() == 128);
```

If the struct does not naturally reach 128 bytes, fill the gap with named
reservations for planned future fields. Count the bytes. Name every one.
The build fails if the assertion fails. This is intentional.

### 3.2 — Structure of Arrays for Large Buffers

Any buffer holding more than 10,000 elements uses Structure of Arrays
layout, not Array of Structures. This is required for cache-coherent
GPU reads. It is not optional.

### 3.3 — WGSL Workgroup Size

Every WGSL compute shader: `@workgroup_size(64, 1, 1)`.
RDNA2 executes exactly 64-thread wavefronts. This is a hardware requirement.
Never 32. Never 128. Never variable.

### 3.4 — The Forensic Corpus Is Append-Only

Every write to the forensic corpus:
- Is followed by `fsync`
- Computes SHA-256 of the raw block
- Stores the first 7 bytes of that hash in `FieldParticle.corpus_hash`
- Never overwrites existing entries
- Never holds data in a heap buffer longer than one processing window

### 3.5 — Test Files Stay in tests/

Any `.iq`, `.pcm`, or `.cf32` file under `tests/` or `examples/` is
blocked from production ingestion by a hard assertion at the ingester
boundary:

```rust
assert!(
    !path.starts_with("tests/") && !path.starts_with("examples/"),
    "Test files must not be used in production"
);
```

This is not a warning. It is a hard error. It returns
`Err(BackendError::InvalidData(...))`.

### 3.6 — The examples/ Shim Rule

`examples/` contains shims only. A shim is a minimal harness that exercises
`src/` code in isolation. It imports from `src/`. It does not contain
production logic. All production code lives in `src/`. If you find yourself
implementing logic in `examples/`, move it to `src/` and import it.

### 3.7 — CPU Reference Implementations

Every WGSL compute shader that produces a numerical result has a CPU
reference implementation. When they disagree, the CPU is correct and the
WGSL is debugged. Both are maintained in parallel. Neither is deleted when
the other is working.

The CPU reference is in `src/reference/`. The WGSL is in `src/shaders/`.

### 3.8 — The `[UNWIRED]` Discipline

A UI control that has no backend wiring displays `[UNWIRED]`.
Never hide unimplemented controls. Never grey them out without labeling.
Never display a default value. `[UNWIRED]` is the only honest label.
It is removed in the same commit that wires the control. Not before.

---

## 4. Track-Specific Rules

### 4.1 — Track 0-D Is the First Deliverable

No other track produces permanent production code until Track 0-D
(the hardware applet) is complete. The applet must be able to detect,
display, and hot-plug every connected device before any signal processing
track begins. If you are assigned a Track A, B, G, or any other track
while Track 0-D is incomplete, implement only the type definitions and
trait interfaces needed by that track, not the logic. The logic waits
for 0-D.

### 4.2 — Track 0-A Types Are the Foundation

`FieldParticle`, `AetherParticle`, `RawIQPoint`, `JuryVerdict` are defined
in Track 0-A. Every other track imports them. Do not redefine them in any
other module. Do not create local aliases. Import from the canonical location.

When Track 0-A types need a new field: that is a Track 0-A task, regardless
of which track discovered the need. File it. Do not add fields to these
structs inside another track's implementation.

### 4.3 — The Dispatch Loop Is a Stub Until Track A1

`dispatch/mod.rs` contains function skeletons with `todo!()` bodies.
They stay as `todo!()` until Track A1 is explicitly assigned. Do not
fill in dispatch logic speculatively. Do not add new `todo!()` stubs
to functions that already have implementations.

### 4.4 — FFT on Point Clouds Only

When you are implementing spatial spectral analysis (Tracks G, H):
the FFT operates on the 3D point cloud geometry in GPU memory.
It does not operate on raw sensor samples. It does not operate on
embeddings returned by the Mamba model. It operates on the spatial
positions and attributes of `AetherParticle` instances after they have
been placed in the scene.

### 4.5 — The Coral Branch Is Independent

The Coral Mamba has its own weights, its own training pipeline, and its
own inference path. It does not share state with GPU Mamba. Do not pass
GPU Mamba intermediate activations into the Coral path. They consume the
same input token stream and produce independent outputs. Their divergence
is the signal. Coupling them destroys the signal.

---

## 5. Error Handling Rules

### 5.1 — No Silent Failures in the Hot Path

Every error in the ingestion pipeline, the space-time Laplacian, or the
PointMamba inference path is logged and surfaced to the hardware applet.
Silent `unwrap()` is not acceptable. Use `expect("descriptive message")`
where a panic is genuinely the right response, or propagate with `?`.

### 5.2 — Hardware Disconnection Is Not an Error

A device disconnecting at runtime is expected behavior. The correct response
is: update the applet to `[DISCONNECTED]`, halt that pipeline branch, log
the timestamp, wait. Do not panic. Do not crash the session. The other
sensors keep recording.

### 5.3 — GPU Crashes During Prototyping Are Acceptable

When Jule is executing experimental shaders (sparse Laplacian, Gram-Schmidt,
cooperative matrix operations): crashes are expected. Document the tile size
or workgroup configuration that caused the crash. Find the stable boundary.
Production machine receives only validated configurations.

---

## 6. Dependency Rules

### 6.1 — No New Dependencies Without Justification

Before adding a crate to `Cargo.toml`, check:
1. Does `wgpu`, `burn`, `bytemuck`, or another existing dependency already
   provide this?
2. Is the crate actively maintained and compatible with NixOS?
3. Is it a pure Rust implementation or does it pull in C dependencies?

C dependencies require an `unsafe` wrapper with documented justification.
Python dependencies require PyO3 and are restricted to Dorothy (Track D)
and training pipelines. Python never appears in the real-time signal path.

### 6.2 — The wgpu Version Is Locked

wgpu 28. Not 27. Not 29. If a new wgpu release fixes something you need,
file it as a migration task. Do not upgrade mid-track without a full
API audit across all shader and pipeline code.

### 6.3 — SPIR-V and Vulkan Extensions Are Permitted

Experimental Vulkan features via SPIR-V are acceptable:
- `VK_KHR_cooperative_matrix` for tensor-core throughput on RDNA
- Subgroup operations for SpMV inner loops
- Any extension that improves the sparse Laplacian computation

Document the extension in the shader file header. Validate that it
degrades gracefully when the extension is unavailable.

---

## 7. The Physical Priors — Never Overwrite These

These constants are derived from physics and ruler measurements.
No ML model output, no learned parameter, no configuration value
can override them. They are compile-time constants.

```rust
pub const SPEED_OF_LIGHT_M_S: f64 = 299_792_458.0;
pub const PLUTO_ANTENNA_BASELINE_M: f32 = 0.012;  // ~12mm, measure and confirm
pub const PICO_CLOCK_HZ: u32 = 150_000_000;
pub const F_MIN_HZ: f32 = 1.0;
pub const F_MAX_HZ: f32 = 700e12;
pub const LOG_RANGE: f32 = 33.18;  // log(F_MAX / F_MIN)
pub const POWERLINE_HZ: f32 = 60.0;
pub const OV9281_FPS: u32 = 120;
pub const OV9281_FRAME_PERIOD_MS: f32 = 8.333;
pub const KNN_K: usize = 20;           // optimal per SI-Mamba ablation
pub const LAPLACIAN_EIGENVECTORS: usize = 4;  // optimal per SI-Mamba ablation
```

If you need a different value for a test: use a local variable in the test.
Do not modify these constants.

---

## 8. The Masterplan Is the Authority

When this file and the masterplan conflict: the masterplan wins.
When a prompt and the masterplan conflict: the masterplan wins.
When your judgment and the masterplan conflict: read the masterplan again,
then use your judgment to clarify or extend it — not to contradict it.

The masterplan is at `SYNESTHESIA_MASTERPLAN.md`.
The status table in Part XVII is updated when milestones complete.
The track structure in Part XII governs what gets built and in what order.
The invariant rules in Part II cannot be waived by any prompt.

---

*The only unrealistic number is infinity.*
*Everything else has a ceiling the math finds.*
