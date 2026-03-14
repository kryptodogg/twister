# Track 0-D Agent Prompt: Hardware Configuration Applet

## Overview

This prompt instructs a coding agent to create the `hardware.slint` applet for Project
Synesthesia. It is one of four Phase 0 tasks. The other three (0-A FieldParticle, 0-B
design tokens, 0-C SAM gate) may or may not be complete when you start — check the
codebase and complete any that you encounter in files you are already touching, per the
Phase 0 incremental rule. If 0-B is not yet done, do it first — `hardware.slint` depends
on `tokens.slint`.

---

## Pre-Flight Block

Write this block verbatim at the top of `ui/hardware.slint` before any other code:

```slint
// === PRE-FLIGHT ===
// Task:           Track 0, Milestone 0-D (Hardware Configuration Applet)
// Files read:     ui/tokens.slint, assets/SKILL-SLINT-MD3.md,
//                 assets/material-1.0/* (full Material Design 3 component library),
//                 ui/toto.slint (reference for component style and structure)
// Files in scope: ui/hardware.slint (new file)
//                 src/hardware/mod.rs (new — SignalBackend wiring)
//                 src/hardware/audio_device.rs (new)
//                 src/hardware/pluto_device.rs (new)
//                 src/hardware/rtl_device.rs (new)
//                 Cargo.toml (add cpal, rtlsdr-rs or soapysdr, libiio bindings if absent)
// Acceptance:     [see acceptance criteria at end of this document]
// Findings:       [fill in after reading the codebase]
// === END PRE-FLIGHT ===
```

---

## Step 0: Repository Cleanup

Before writing any new code, delete the following. Do not ask for confirmation — these
are test and example artifacts that must not be referenced in production code.

**Delete all** `.slint` files except `ui/toto.slint` and `ui/tokens.slint`. If any other
`.slint` file exists (widgets, demos, experiments), remove it.

**Delete all** files under `examples/` except the `examples/toto` example (the `cargo run
--example toto` entry point). If `examples/toto/` is a directory, keep it entirely. If
it is a single `examples/toto.rs` file, keep it. Remove everything else in `examples/`.

**Delete all** files under `tests/` entirely. Tests that reference these files will also
need their references removed. If removing a test file breaks a `#[cfg(test)]` block in
production code, remove the test block and note it in your pre-flight Findings.

**Delete all** files under `benches/` if that directory exists.

After deletion, run `cargo check`. It must compile cleanly before you write any new code.
If it does not compile cleanly after deletion, diagnose and fix the broken references
before proceeding.

---

## Step 1: Read the Material Design 3 Reference

Open `assets/SKILL-SLINT-MD3.md` and `assets/material-1.0/` before writing any `.slint`
code. The component library in `assets/material-1.0/` is the source of truth for every
visual element in this applet. The Toto applet (`ui/toto.slint`) is the established
reference for how these components are used in this project — match its visual language
exactly.

Rules that apply to every `.slint` file:
- Import `ui/tokens.slint` at the top. Never use hex color literals; use `Colors.*` tokens.
- Use only components and layout primitives found in `assets/material-1.0/` or the Slint
  standard library. Do not invent new components.
- Any control that is not wired to real hardware must display a `[MOCK]` badge — a small
  teal label using `Colors.Tertiary` — in its top-right corner. Remove the badge in the
  same milestone as the Rust wiring, not separately.
- `todo!()` and `unimplemented!()` are build failures, not acceptable stubs.

---

## Step 2: Design the Hardware Applet Structure

`hardware.slint` is a standalone applet that will later be integrated as a flyout panel
accessible from the main system UI. For now, it opens as its own window (`cargo run
--example hardware`). Add a matching `examples/hardware.rs` entry point.

The applet has two levels: a **Device Gallery** (the default view) and a
**Device Configuration Panel** (shown when a card is clicked).

### Device Gallery

The gallery shows three hardware cards arranged in a vertical list or a 3-column grid
(choose the layout that best fits the MD3 card component from `assets/material-1.0/`).
Each card displays:

The device name as the card headline (RTL-SDR, Pluto+, Soundcard). A status indicator:
🟢 Connected, 🟡 Detected but unconfigured, 🔴 Not detected. The indicator is read from
a Rust property exposed to Slint — it is not hardcoded. A one-line capability summary as
the card subtitle: "RX only · 24–1766 MHz" for RTL-SDR, "TX + RX · 70 MHz – 6 GHz" for
Pluto+, "TX + RX audio · 20 Hz – 96 kHz" for Soundcard. A `[MOCK]` badge until the Rust
backend detection is wired.

Clicking a card opens the configuration panel for that device. The gallery and the
configuration panel must not both be visible simultaneously — the panel replaces the
gallery in the same window area.

### Device Configuration Panel

Every device shares a common panel structure with a back button (←) in the top-left that
returns to the gallery. Below the back button: the device name as the panel title, the
status indicator, and a **Test** button and a **Save Defaults** button in the top-right.
Below that: a scrollable content area with the device-specific controls described below.

---

## Step 3: Device-Specific Controls

### RTL-SDR Card

The RTL-SDR is receive-only. Its configuration panel contains the following controls.

A **Frequency Tuner** consisting of a numeric input field showing the current frequency
and a unit selector dropdown (Hz, kHz, MHz, GHz). The numeric field accepts decimal
values. When the unit is changed, the displayed number scales accordingly (1,000,000 Hz
= 1.0 MHz = 0.001 GHz). The tuner range is 24 Hz to 1766 MHz. Tuning outside this range
produces a visible warning but does not prevent input — the Rust layer will clamp. There
is also a coarse-tuning slider below the numeric input covering the full range on a
logarithmic scale, synchronized with the numeric field.

A **Sample Rate** dropdown with common values: 0.25 MSPS, 1.0 MSPS, 2.048 MSPS,
2.4 MSPS, 3.2 MSPS.

A **Gain Mode** toggle between Auto and Manual. When Manual is selected, a gain slider
appears (0–49 dB in 1 dB steps, matching the RTL-SDR gain table).

A **PPM Correction** numeric input (integer, -100 to +100).

A **Test** button labeled "Start RX". When pressed, the applet shows a live signal
strength indicator (a horizontal bar whose width is proportional to the measured signal
power in dB) updating at 10 Hz. It also shows the currently detected peak frequency
within ±500 kHz of the tuned frequency. A second press labeled "Stop RX" halts reception.

### Pluto+ Card

The Pluto+ supports both TX and RX. Its configuration panel contains the following.

An **RX/TX Mode** tab or toggle at the top of the scrollable area. The controls below
change depending on which mode is active.

**Shared controls** (always visible regardless of mode):

The same Frequency Tuner as the RTL-SDR, but the range is 70 MHz to 6 GHz. Add a set of
**Band Presets** as a horizontal row of small chips below the tuner: 433 MHz, 915 MHz,
1090 MHz (ADS-B), 2.4 GHz, 5.8 GHz. Pressing a chip fills the frequency tuner with that
value. These are convenience shortcuts, not the only allowed values.

A **Sample Rate** dropdown: 0.52 MSPS, 1 MSPS, 2 MSPS, 4 MSPS, 10 MSPS, 20 MSPS,
40 MSPS, 56 MSPS.

**RX-mode specific controls**:

A **Gain Mode** toggle (Auto / Manual). Manual shows a gain slider (0–73 dB).

A **Test** button labeled "Start RX" that shows the same live signal strength indicator
as the RTL-SDR.

**TX-mode specific controls**:

A **Waveform Mode** selector. This is a horizontal segmented control with four options:

*CW (Continuous Wave)*: Transmits a continuous unmodulated carrier at the specified
frequency. No additional controls beyond the frequency tuner and power level. This is the
mode the user described as "I set it to 1 MHz, it just transmits a continuous 1 MHz wave
that I can measure with SDR++." Pressing the Transmit button starts the carrier;
pressing again (now labeled Stop) halts it.

*Tone (Sinc-filtered)*: Transmits a tone synthesized from a sinc-interpolated waveform.
Shows a **Tone Frequency** field (separate from the carrier frequency — this is the
baseband modulation frequency). Shows a **Sinc Kernel Width** selector (number of
lobes: 2, 4, 8, 16) that controls the anti-aliasing quality of the sinc interpolator.
Shows a rolloff visualization (a small static plot showing the frequency response of the
selected sinc kernel — this can be a simplified static SVG per kernel width, not a
live-rendered plot). The tone is upconverted to the carrier frequency on the Pluto+.

*W-OFDM (Wavelet OFDM)*: Transmits a Wavelet OFDM burst using the Daubechies IDWT
synthesis from Track A-WOFDM. Shows a **Wavelet Family** dropdown (Daubechies-4,
Daubechies-8). Shows a **Symbol Count** numeric field (how many OFDM symbols to transmit
in the burst, 1–256). Shows a **Guard Interval** toggle that defaults to OFF — the whole
point of W-OFDM is no guard intervals; the toggle exists to allow A/B comparison with
standard OFDM in tests. When OFF, a small info label reads "Compact support — no ISI, no
guard interval overhead." When ON, a standard cyclic prefix is prepended and the label
reads "Guard interval ON — standard OFDM comparison mode." A **Transmit Burst** button
sends one burst; a **Loop** toggle makes it repeat continuously.

*File (IQ)*: Load a `.iq` or `.cf32` file from disk and transmit its contents. Shows a
file picker button, the selected filename, the file size and inferred sample count (based
on 2 × f32 = 8 bytes per complex sample), and an estimated duration at the current sample
rate. A warning banner appears if the file size is not divisible by 8 ("Not a valid IQ
file — size must be divisible by 8 bytes"). The play/loop button behavior matches the
W-OFDM burst controls.

**TX Power Level**: A slider from -89.75 dBm to 0 dBm in 0.25 dB steps (AD9363 TX
attenuation range). Shows the value numerically alongside the slider.

**TX Test Button**: Labeled "Transmit" in CW and Tone modes, "Transmit Burst" in W-OFDM
mode, "Transmit File" in File mode. When active, the button turns red and relabels to
"Stop". A [LIVE TX] badge in red pulses next to the device status indicator while any TX
is in progress.

### Soundcard Card

The Soundcard supports both playback (TX) and recording (RX). Its configuration panel
contains the following.

A **Mode** toggle: Playback and Record, with device pickers for each.

**Playback controls**:

A **Playback Device** dropdown listing available audio output devices (populated from the
CPAL device enumeration). Shows the device name and its maximum supported sample rate.

A **Sample Rate** dropdown: 44100 Hz, 48000 Hz, 88200 Hz, 96000 Hz, 192000 Hz (filtered
to rates supported by the selected device).

A **Bit Depth** selector: 16-bit, 24-bit, 32-bit float.

A **Waveform Mode** selector — the same four options as the Pluto+ TX panel (CW Tone,
Sinc Tone, W-OFDM, File), but the carrier frequency for the Soundcard is in the audio
range (20 Hz – 96 kHz, matching the sample rate limit). The Tone Frequency field for
sinc mode operates in the same audio range. The W-OFDM wavelet synthesis uses the same
GPU IDWT shader as the Pluto+ — the only difference is the `SignalBackend` destination.
This is the Backend::Audio proving ground described throughout the roadmap.

A **Volume** slider (0%–100%).

A **Play / Stop** button.

**Record controls**:

A **Recording Device** dropdown listing available audio input devices.

A **Test Record** button that records 3 seconds of audio and shows a simple peak level
meter (a horizontal bar updated in real time during the recording, showing the maximum
absolute sample value per 100ms window as a fraction of full scale). After the recording
completes, shows the peak level achieved and whether any clipping was detected. Writes the
recording to `assets/test_capture_YYYYMMDDTHHMMSS.pcm` — this is the Backend::File
output requirement. It is not optional.

---

## Step 4: Rust Backend Wiring

Create the following Rust module structure:

`src/hardware/mod.rs` — exports `HardwareRegistry`, the struct that discovers and holds
references to all connected devices. Implements a `scan()` method that enumerates
available devices and returns their detected status. Exposes device status to Slint via
thread-safe properties.

`src/hardware/audio_device.rs` — wraps CPAL for device enumeration, playback, and
recording. Implements the `SignalBackend` trait for playback. The recording path writes to
`Backend::File` automatically (this is non-optional).

`src/hardware/pluto_device.rs` — wraps libiio for Pluto+ TX and RX. Implements the
`SignalBackend` trait. The file path: write the IQ of every TX session to
`assets/tx_pluto_YYYYMMDDTHHMMSS.iq` automatically as `Backend::File` output.

`src/hardware/rtl_device.rs` — wraps the RTL-SDR library for RX. Implements a streaming
interface that feeds the live signal strength indicator. Writes recorded IQ to
`assets/rx_rtl_YYYYMMDDTHHMMSS.iq` automatically as Backend::File output.

If `SignalBackend` does not yet exist in the codebase, define it here:

```rust
// src/hardware/mod.rs
pub trait SignalBackend: Send {
    /// Write interleaved f32 complex samples (alternating I, Q) to the backend.
    fn write_iq(&mut self, samples: &[f32]) -> Result<(), BackendError>;

    /// Write interleaved f32 real samples (mono PCM) to the backend.
    fn write_pcm(&mut self, samples: &[f32]) -> Result<(), BackendError>;

    /// Flush any buffered output and finalize (e.g., close file, stop TX stream).
    fn flush(&mut self) -> Result<(), BackendError>;

    /// Human-readable description for logging: "Audio(WASAPI:Realtek)", "Pluto+(192.168.2.1)", etc.
    fn describe(&self) -> &str;
}

pub enum BackendError {
    DeviceNotFound(String),
    ConfigurationError(String),
    IoError(String),
    InvalidData(String),
}
```

The `SignalBackend` implementations for each device must be compile-time stubs (returning
`Err(BackendError::DeviceNotFound(...))` with a descriptive message) if the underlying
hardware library is not yet linked. The UI shows `[MOCK]` badges in that case. When the
hardware library is linked and a device is detected, the stub is replaced by the real
implementation and the `[MOCK]` badge disappears. The removal of the `[MOCK]` badge
and the successful wiring of the backend must happen in the same commit — a `[MOCK]` badge
on a wired control is a lie, and a missing badge on an unwired control is a worse lie.

**Timestamp rule**: Use `QueryPerformanceCounter` via `windows-sys` for all file
timestamps in hardware output filenames and in the `timestamp_us` field of any
FieldParticle produced during hardware tests. `SystemTime::now()` is not acceptable.
The session epoch QPC is captured once at process start and stored in a global. All
subsequent timestamps are offsets from that epoch in microseconds.

**No test data in production**: Any `.iq`, `.pcm`, or `.cf32` file under `tests/` or
`examples/` is a test artifact and must never be opened by `RFIngester`, `AudioIngester`,
or any production code path. Add an assertion: if the configured file path contains
`tests/` or `examples/` as a path component, return
`Err(BackendError::InvalidData("Test files must not be used in production"))` immediately.
This is the No-Mock-In-Production rule applied to file paths.

---

## Step 5: Waveform Synthesis Plumbing

The CW, Tone, and W-OFDM modes all require waveform synthesis before transmission. For
this milestone, implement synthesis on the CPU (not the GPU IDWT shader — that is Track
A-WOFDM). The CPU implementation exists solely to prove the `SignalBackend` plumbing
end-to-end. When Track A-WOFDM is complete, it replaces the CPU implementation behind
the same interface without changing the Slint UI.

**CW synthesis**: Fill a buffer of N samples with `i_sample = 1.0, q_sample = 0.0`
(a baseband carrier at DC, upconverted to the center frequency by the hardware). For the
Soundcard, fill with `sample = sin(2π · f_tone / f_sample_rate · n)` for sample index n.

**Sinc-filtered tone synthesis**: Convolve a pure tone with a windowed sinc kernel of the
selected width. For the Soundcard CW/Tone mode, this produces band-limited tone output.
For the Pluto+, this produces a band-limited baseband waveform that the AD9363 upconverts.
The rolloff visualization in the UI can be a pre-computed SVG per kernel width — do not
attempt live FFT-rendered spectrum plots in this milestone.

**W-OFDM synthesis (CPU placeholder)**: For now, generate a simple multi-tone signal
using an IFFT (not the Daubechies IDWT). Set a flag in the output metadata that marks
this as "CPU IFFT placeholder, not true W-OFDM." The W-OFDM milestone (Track A-WOFDM)
replaces this with the GPU IDWT shader. The `[MOCK]` badge on the W-OFDM mode controls
must remain until Track A-WOFDM is complete and wired.

---

## Step 6: Examples Entry Point

Create `examples/hardware.rs`:

```rust
// examples/hardware.rs
//
// Launch the Hardware Configuration Applet standalone.
// Usage: cargo run --example hardware
//
// This example is the correct way to develop and test the hardware applet.
// It must never be used as a data source by production code.

fn main() {
    // Initialize hardware registry (scans for connected devices)
    // Launch Slint window running hardware.slint
    // Event loop
}
```

---

## Acceptance Criteria

All of the following must be true before this milestone is considered complete.

**Cleanup**: `cargo check` passes after deleting all examples except `toto` and
`hardware`, all tests, all `.slint` files except `toto.slint` and `tokens.slint`.

**Launch**: `cargo run --example hardware` opens a window within 3 seconds on a Windows
11 machine with the RX 6700 XT. No crash, no panic.

**Gallery**: All three hardware cards render with correct names, subtitles, and a status
indicator. Cards are visually consistent with the Toto applet — same typography, same
card radius, same color tokens, same Material Design 3 component language. `[MOCK]` badges
are present on status indicators until device detection is wired.

**Navigation**: Clicking any card opens its configuration panel. Pressing the back button
(←) returns to the gallery. The transition is immediate (no required animation at this
milestone, but it must not flicker or show both panels simultaneously).

**RTL-SDR panel**: All controls render (frequency tuner, sample rate, gain, PPM). The
Start RX button calls the RTL-SDR backend; the live signal strength bar updates when RX
is active. If no RTL-SDR is connected, the button is disabled or shows a `[DEVICE NOT
FOUND]` message — it does not panic.

**Pluto+ panel**: All four waveform modes render their specific controls (CW, Sinc Tone,
W-OFDM, File). Setting the frequency tuner to 1.0 MHz, selecting CW mode, and pressing
Transmit causes the Pluto+ to transmit a continuous 1 MHz carrier measurable in SDR++ at
1 MHz. The [LIVE TX] badge appears while transmission is active. Pressing Stop halts
transmission. The TX session IQ is written to `assets/tx_pluto_*.iq` automatically. If
no Pluto+ is connected, the Transmit button is disabled and shows `[DEVICE NOT FOUND]` —
it does not panic.

**Soundcard panel**: The Playback Device dropdown is populated with real system audio
devices. Setting frequency to 440 Hz in CW mode and pressing Play produces a 440 Hz sine
wave through the selected output device. Pressing Stop halts it. The Test Record button
records 3 seconds and writes to `assets/test_capture_*.pcm`.

**W-OFDM mode**: Selecting W-OFDM mode and transmitting produces a visually distinct
spectrum in SDR++ (or a spectrum analyzer plugin) — recognizably different from the CW
mode single spike. The `[MOCK]` badge is present on W-OFDM controls to indicate the CPU
IFFT placeholder, not the true Daubechies IDWT synthesis.

**File integrity**: Every TX and RX session automatically writes a file to `assets/`.
The file exists on disk after the test and is not zero bytes. The filename includes a
QPC-derived timestamp in ISO 8601 format.

**No test data in production**: The assertion that blocks `tests/` and `examples/` paths
from being opened by production ingesters is present and tested — attempting to pass
a file from either directory to `RFIngester::from_file()` returns `Err(...)`, not Ok.

**Phase 0 incremental rule**: If during this milestone you encounter any missing Phase 0
items in files you are editing (missing `const _: () = assert!(...)` size checks, missing
`tokens.slint` imports in any `.slint` file, the `assets/hardware_gate.txt` verification
absent from wgpu initialization), complete them in the same pass. Document them in the
pre-flight Findings block.

---

## What This Applet is Not

This applet is not the Kansas TUI. Kansas is the text-based diagnostic interface for
ongoing system monitoring. The Hardware applet is the setup and test surface used at the
beginning of a session to configure devices, verify they are working, and establish
defaults. Once devices are configured and the Hardware applet is closed, Kansas takes over
for ongoing monitoring. They serve different moments in the workflow.

This applet does not replace the Toto or Chronos applets. It will eventually be accessible
as a flyout from those applets (triggered by a hardware status indicator or a settings
icon), but for now it is a standalone window. The flyout integration is a separate
milestone.

The W-OFDM, fractal, and DPC waveform synthesis modes in this applet are UI stubs with
CPU placeholders. They exist so the full waveform API surface is designed and validated
before the GPU shader implementations are written. When Track A-WOFDM is complete, it
slots into the existing interface without a UI redesign.

---

*Agent assignment: Track 0, Milestone 0-D · Reads: toto.slint, tokens.slint,
SKILL-SLINT-MD3.md, assets/material-1.0/ · In scope: ui/hardware.slint,
src/hardware/*.rs, examples/hardware.rs, Cargo.toml · Acceptance: see above*
