# SIREN v0.2 — GPU-First Acoustic Active Denial

## The Core Architectural Principle

v0.1 had this backwards: the CPU owned all synthesis data and the GPU
was an optional accessory. v0.2 inverts this to match the actual hardware.

Your RX 6700 XT has 12 GB of GDDR6 at **384 GB/s**. That is the primary
address space. The CPU has 64 GB of DDR4 at around 51 GB/s — significantly
slower for bulk compute. With Smart Access Memory (ReBAR) enabled, the CPU
can write directly into GDDR6 at ~32 GB/s over PCIe 4.0. This is the
architectural sweet spot: the CPU's job is to write small control structs
into VRAM and then get out of the way while the GPU's 1536 shader processors
handle synthesis in parallel.

### Comparison: Your System vs Apple Silicon

| Path                           | Bandwidth         |
|--------------------------------|-------------------|
| RX 6700 XT VRAM (internal)     | 384 GB/s          |
| CPU → VRAM via SAM (PCIe 4.0)  | ~32 GB/s          |
| Apple M1 unified memory        | 68 GB/s           |
| Apple M2 Pro unified memory    | 200 GB/s          |
| Apple M1/M2 SSD paging         | ~7 GB/s ideal     |

The critical insight: once an Apple Silicon Mac's model exceeds the unified
memory ceiling, it falls off a cliff to SSD paging. Your system has a
graceful degradation path: VRAM → system RAM via SAM → SSD. For models
that fit in VRAM (anything under ~12 GB), you have a genuine bandwidth
advantage over all base Apple Silicon chips.

## Data Flow

```
[Microphone]
    │ PCM samples via cpal callback (real-time, never blocks)
    ▼
[crossbeam channel] — lock-free, CPU-side only
    │
    ▼
[Dispatch thread every 50ms]
    │ 1. Drain samples from channel
    │ 2. CPU FFT (rustfft, ~10µs for 2048 pts)
    │ 3. Update SynthParams struct
    │ 4. queue.write_buffer() → writes 48 bytes directly to GDDR6 via SAM
    │ 5. Submit WGSL compute dispatch: 512 threads × 1 frame each = 512 frames
    │ 6. Copy VRAM output → VRAM readback buffer (VRAM→VRAM at 384 GB/s)
    │ 7. Map readback buffer → CPU reads synthesized frames
    ▼
[AppState::output_frames Mutex]
    │ Refilled with fresh GPU output
    ▼
[cpal output callback] — reads frames, writes to hardware DMA buffer
    │
    ▼
[Speakers / 7.1 surround]
```

## WGSL Synthesis Shader

The key to the GPU synthesis is analytical phase computation. Each of the
512 threads handles exactly one audio frame, and computes its output using:

```wgsl
let phase_at_frame = (initial_phase + f32(frame_idx) * phase_inc) % TAU;
```

Because this formula depends only on the frame index (not on adjacent
frames), there are zero inter-thread dependencies. All 512 threads execute
simultaneously. Compare this to the v0.1 approach where synthesis ran
as a sequential for-loop on the CPU, executing 512 iterations one at a time.

## Denial Modes

All synthesis happens in the WGSL shader. The CPU writes a `mode` u32 into
SynthParams and the shader's switch statement selects the appropriate synthesis.

- **Off** — silence, monitoring only
- **Anti-Phase** — inverted sine at the detected fundamental; creates a
  destructive interference null at the dominant frequency
- **Noise Mask** — broadband pink noise via LCG; per-frame seeding avoids
  sequential dependencies between threads  
- **Pure Tone** — in-phase sine for confirming frequency lock; you hear
  beating when the tone is near the interferer
- **Sweep** — 20 Hz → 8 kHz scanning tone; finds interference nodes by ear

## 7.1 Channel Routing

The `channel_weight()` function in the shader assigns each channel a gain
coefficient. For 7.1 (8 channels): front pair at 0.4, side pair at 0.35,
rear pair at 0.35, center at 0.2, LFE at 0.1. The LFE subwoofer is reduced
because most interference is in the midrange, not sub-bass. The shader
gracefully falls back to 5.1, stereo, or mono based on `n_channels`.

## Stage 1 Upgrade: IQUMamba Integration

The dispatch loop is designed so that the CPU FFT is the only part that
needs to change when the Mamba autoencoder lands. The input path and output
path remain identical. The FFT is replaced by:

1. Write IQ samples into an `input_buffer` in VRAM (same SAM write path)
2. Dispatch the Mamba encoder compute passes (VRAM in → latent VRAM buffer)
3. The synthesis shader reads from the latent buffer instead of SynthParams

The latent buffer replaces `denial_freq_hz` + `mode` with a full learned
representation of the signal structure, enabling phase-coherent separation
of heterodyned carriers. The output path (readback → AppState → cpal) does
not change at all.

## Building

```bash
cargo build --release
```

Requires: Rust 1.75+, Vulkan-capable GPU (RX 6700 XT, any RDNA2/3, or Nvidia
with Vulkan), cpal-compatible audio device.

For best performance on AMD, ensure the amdvlk or mesa radv Vulkan driver is
installed. On Linux with Mesa, SAM is enabled by default when ReBAR is active
in BIOS.
# twister
