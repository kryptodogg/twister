# Project Synesthesia — Modulation Theory Addendum
## W-OFDM, Fractal Modulation, Dirty Paper Coding, Extreme QAM, Impulse Radio

**Addendum to**: `ROADMAP.md` + `ROADMAP_ADDENDUM_physics_haptics.md`  
**Scope**: Advanced modulation schemes exploiting bit-depth for throughput,
Dirty Paper Coding as a theoretical basis for H3, and impulse/rpitx experiments.

---

## Hardware Reality Check: 12-Bit, Not 24-Bit

The Pluto+ uses an **AD9363 transceiver: 12-bit ADC/DAC**.  
Theoretical SNR ceiling: ~72 dB (not 144 dB).  
Practical SNR after analog noise floor: likely 50–60 dB in a real room.

The 24-bit figure applies to a high-end audio card (e.g., ESS Sabre DAC) on the
acoustic side — and that is exactly where algorithm development starts. The
**Variable Backend Rule** (defined in `ROADMAP.md` agent hard rules) governs
every milestone in this addendum:

- Prove on `Backend::Audio` (24-bit, no licensing, bugs are obvious)
- Promote to `Backend::Pluto` (12-bit RF, real channel conditions)
- `Backend::File` output required from every test run

Milestones with separate "acoustic" and "RF" variants have been collapsed into
single parameterized milestones. There is one algorithm, one shader, one acceptance
test — run twice with different backends. The soundcard is the proving ground;
the Pluto+ is the deployment target.

---

## Addition to Track A — W-OFDM Synthesis (Cyclone Extension)

### A-WOFDM: Wavelet OFDM Transmit Synthesis

Cyclone already uses Daubechies wavelets (6 decomposition levels) for *receive-side*
gesture feature extraction following WiGrus. The natural extension is using the same
wavelet basis for *transmit-side* symbol synthesis — replacing the standard IFFT-based
OFDM waveform generator with an Inverse Discrete Wavelet Transform (IDWT).

**Why this matters**:  
Standard OFDM uses sinc-based subcarriers that have infinite time-domain support —
they "ring" into adjacent symbols. To prevent inter-symbol interference (ISI),
OFDM inserts a Cyclic Prefix (CP) between every symbol: a dead-zone guard interval
that wastes roughly 20% of available air time doing nothing.

Daubechies wavelets have *compact support* — they are mathematically zero outside
a finite time window. No ringing. No ISI. No guard interval needed. The 20%
overhead disappears, and symbols can be packed back-to-back.

**Implementation path**:
- WGSL compute shader: IDWT synthesis of W-OFDM symbols on the GPU
- Input: frequency-domain symbol vector
- Output: time-domain IQ/PCM waveform, continuous, no guard intervals
- `SignalBackend` selects destination: sound card DAC, libiio TX buffer, or file
- Same shader, same algorithm, backend is a runtime parameter — never hardcoded

**Acceptance (Backend::Audio — prove here first)**:
- Loopback via microphone. W-OFDM symbol count in a fixed time window must exceed
  standard OFDM by ≥ 20% (the guard interval recovery).

**Acceptance (Backend::Pluto — after Audio is green)**:
- Same test via Pluto+ TX → RX loopback. Result must match Audio within measurement
  noise. Divergence means hardware interface bug, not algorithm bug.

**Acceptance (Backend::File — always)**:
- IQ/PCM file written alongside every test run as regression artifact.

---

## Addition to Track H — Extreme-Order QAM

### H-QAM: Constellation Depth Exploitation

Standard protocols cap at 1024-QAM or 4096-QAM because commodity receivers have
10–12 bit resolution. At 12-bit depth (Pluto+), the theoretical ceiling is:

```
Max QAM order ≈ 2^(SNR_dB / 3) for square constellations
At 55 dB practical SNR: 2^(55/3) ≈ 2^18 = 262144-QAM theoretical
At 45 dB (conservative): 2^(45/3) = 2^15 = 32768-QAM
```

A practical target of **4096-QAM to 16384-QAM** is achievable in a controlled
room environment where the WRF-GS model has characterized the channel.

**The key enabler**: WRF-GS gives an accurate channel model. Standard high-order
QAM fails because the receiver doesn't know which points in the constellation got
smeared by multipath. With a pre-characterized channel, the TX can pre-compensate
— the constellation arrives at the RX with clean separation even in a reflective room.

**Why this matters for Project Synesthesia specifically**: The goal is not data
throughput for its own sake. Extreme-order QAM is the mechanism by which the
counter-waveform TX (Track H) can carry a *cloak signal* on the same waveform as
a *null signal*, with a third layer carrying *channel probe packets* — all on the
same transmission, distinguished only by constellation depth layers that a standard
receiver cannot resolve.

**H-QAM1 — Constellation depth profiling**
- Transmit a sweep of QAM orders (64 → 256 → 1024 → 4096) on the Pluto+
- Receive with the same Pluto+ in loopback or a second SDR
- Plot EVM (Error Vector Magnitude) vs QAM order to find the practical ceiling
  for your specific room, cable, and antenna configuration
- **Acceptance**: Identify the highest stable QAM order where EVM < -30 dB.
  Document as the "room's QAM ceiling" — use in all subsequent H milestones.

**H-QAM2 — Multi-layer symbol packing**
- Encode three independent data streams into a single symbol using QAM depth layers:
  - Layer 1 (coarse, visible to any receiver): navigation/beacon data
  - Layer 2 (medium, visible to 12-bit receiver): WRF-GS field updates
  - Layer 3 (fine, visible only at full SNR): private channel / high-precision control
- **Acceptance**: Three independent decoders at different precision thresholds each
  correctly recover their layer from a single transmitted symbol stream.

---

## Major Addition to Track H3 — Dirty Paper Coding

### H3-DPC: Dirty Paper Coding (replace naive H3 gradient descent framing)

**H3 in the base roadmap** describes the counter-waveform synthesis as gradient
descent minimizing field strength at a target. This works, but it treats room
reflections as interference to fight. Dirty Paper Coding (DPC) inverts this:
reflections become *collaborators*.

**Shannon-Wolfowitz DPC theorem**: If the transmitter has *non-causal* knowledge
of the interference in the channel, it can achieve the same capacity as if the
interference did not exist — regardless of how strong the interference is.

In Project Synesthesia, the "interference" is the room's multipath. The Pluto+
has exactly this non-causal knowledge: WRF-GS + PINN has already characterized
every significant reflection path. The TX can therefore pre-distort the waveform
so that by the time it has bounced off all the walls, it arrives at the target
as a clean, uncorrupted signal — as if the room were anechoic.

**The elegant inversion**: instead of synthesizing a waveform and hoping it
survives the room, you synthesize a waveform that *needs* the room to be correct.
The room is part of the codec.

**H3-DPC1 — Costa precoding (practical DPC)**
- Implement Tomlinson-Harashima Precoding (THP) as the practical DPC variant
  (exact DPC requires exponential codebook search; THP approximates it with
  a simple modulo operation at the TX)
- Input: desired received waveform at target, WRF-GS channel matrix H
- Output: pre-coded TX waveform x such that H·x ≈ desired at target
- **Acceptance**: In a single-reflector test environment, THP-precoded TX
  produces ≥ 6 dB SNR improvement at the target compared to naive IFFT TX.

**H3-DPC2 — Room-as-codec null synthesis**
- The biometric cloak null (Track I2) is the primary use case for DPC
- Instead of fighting multipath to create a null, use multipath to create it
- Synthesize a waveform whose bounce paths destructively interfere specifically
  at the body-occupied volume, constructively everywhere else
- The room's walls do the work; the Pluto+ just provides the seed signal
- **Acceptance**: Null depth at target volume ≥ 15 dB (same metric as I2),
  but achieved with 10× lower TX power than naive counter-waveform approach.

---

## Addition to Track H — Fractal Modulation

### H-FRAC: Fractal Waveform Encoding (Wavelets Inside Wavelets)

The 12-bit amplitude resolution supports encoding data at multiple simultaneous
scales on a single waveform — the macro layer visible to any SDR, the micro layer
visible only to a receiver with sufficient bit depth and knowledge of the encoding.

**Structure**:
```
Macro layer (bits 12..7): standard modulation — slow, high-power, visible to all
Micro layer (bits 6..1):  high-frequency micro-wavelets riding the macro envelope
                           visible only to a receiver with ≥12-bit resolution
                           and knowledge of the carrier wavelet basis
```

The micro-wavelets are selected to ride the *slope* of the macro wavelet — they
exist on the amplitude surface of the carrier, not as additive interference.
To a 6–8 bit receiver, they are invisible inside the quantization noise floor.

**Relationship to W-OFDM**: The macro layer *is* the W-OFDM frame. The micro
layer uses a different wavelet family at a higher decomposition level. Daubechies-4
for macro, Daubechies-8 for micro, orthogonal decomposition ensures they don't
interfere with each other's data.

**H-FRAC1 — GPU IDWT fractal synthesis shader**
- WGSL compute shader: accepts macro symbol vector + micro symbol vector,
  synthesizes a single IQ/PCM waveform containing both
- Independent IDWT per layer at different wavelet scales (Daubechies-4 macro,
  Daubechies-8 micro — orthogonal decomposition, no inter-layer interference)
- Superposition with appropriate amplitude scaling for the micro layer
- `SignalBackend` selects output destination — never hardcoded

**Acceptance (Backend::Audio — prove here first)**:
- Macro layer: inaudible sub-100 Hz tone. Micro layer: 18–22 kHz ultrasonic channel.
- Both encoded as IDWT fractal on the same PCM stream to the sound card.
- Verify: (a) microphone + decoder recovers micro-layer data correctly,
  (b) a standard audio decoder cannot see the micro layer (below its noise floor),
  (c) the 24-bit headroom makes the amplitude separation unambiguous in spectrum analysis.

**Acceptance (Backend::Pluto — after Audio is green)**:
- Same fractal structure on RF IQ. Macro layer visible to a 6-bit equivalent decoder.
  Micro layer invisible to same decoder, recoverable at full 12-bit resolution.
- File output written alongside both test runs.

---

## New Phase K — Impulse Radio / rpitx Experiments

**Prerequisites**: Track H complete, Pluto+ TX pipeline proven  
**Hardware**: Raspberry Pi (Toto node), no additional hardware required for rpitx

**These are experiments, not production systems.** rpitx turns the Pi's GPIO
clock into a crude SDR transmitter by manipulating the hardware clock's frequency.
It is not precision hardware. It is useful for proofs-of-concept, range testing,
and understanding propagation at frequencies where the Pluto+ is less convenient.

**Legal note**: rpitx transmits on frequencies that require licensing in most
jurisdictions. All experiments must be conducted in a shielded environment
or on frequencies where Part 15 / amateur radio authorization applies.

**K1 — rpitx W-OFDM proof-of-concept**
- Implement W-OFDM waveform synthesis on the Pi (CPU version of the IDWT shader,
  not GPU — the Pi doesn't have a wgpu-capable GPU)
- Transmit via rpitx, receive via Pluto+
- Goal: verify that W-OFDM's guard-interval elimination survives real hardware
  imperfections (phase noise, frequency drift) that a perfect simulation would hide
- **Acceptance**: Receive and decode W-OFDM symbols from rpitx with EVM < -20 dB
  over 1 meter in a shielded environment.

**K2 — Impulse radio ranging**
- Impulse radio (IR-UWB concept): transmit very short pulses (< 1 ns equivalent
  pulse width via rpitx frequency hopping approximation)
- Measure time-of-flight to reflectors in the room
- Compare measured ranges against WRF-GS scene geometry
- Goal: validate that the WRF-GS model's reflector positions match physical reality
- **Acceptance**: Measured reflector distances match WRF-GS scene within ±10 cm.

**K3 — Multi-node fractal mesh**
- Two Toto Raspberry Pi nodes, each running rpitx fractal TX
- Each node transmits a macro layer beacon + micro layer data
- Pluto+ receives both simultaneously, separates by wavelet basis
- Goal: demonstrate frequency reuse — two transmitters on the same carrier,
  distinguished by wavelet family rather than frequency separation
- **Acceptance**: Pluto+ correctly decodes both nodes' micro-layer data streams
  with < 5% symbol error rate despite transmitting on overlapping frequencies.

---

## Shannon-Hartley Context for the Full Stack

For reference, where each technique sits against the Shannon limit:

```
Standard 64-QAM OFDM with CP:
  Efficiency: ~4 bits/s/Hz, ~80% of theoretical (CP wastes 20%)
  
W-OFDM (no CP) at 64-QAM:
  Efficiency: ~5 bits/s/Hz, ~100% of theoretical for this SNR
  
W-OFDM at 4096-QAM (achievable at 55 dB SNR):
  Efficiency: ~10 bits/s/Hz at same bandwidth
  
Fractal W-OFDM (macro + micro layers):
  Effective efficiency: depends on SNR split between layers
  At 12-bit hardware, ~12–14 bits/s/Hz practical ceiling
  
Dirty Paper Coding adds no bits/s/Hz but removes multipath SNR penalty:
  Effective gain: equivalent to 6–10 dB SNR improvement in a reflective room
  Which translates to ~2–3 extra bits/symbol in the presence of strong multipath
  
Shannon limit for Pluto+ at 50 dB SNR, 1 MHz bandwidth:
  C = B·log₂(1 + SNR) = 1 MHz · log₂(100001) ≈ 16.6 Mbits/s
  Current protocols achieve ~4–6 Mbits/s in this window
  W-OFDM + DPC + fractal targets ~10–12 Mbits/s — within factor of 2 of Shannon
```

The full stack is not theoretically exotic — it is a principled path from current
practice toward the Shannon ceiling using tools that are implementable with the
hardware already in the system.

---

## Revised Status Additions

| Track | Status | Blocking issue |
|-------|--------|----------------|
| A-WOFDM W-OFDM synthesis | 🔴 Not started | Needs A1 (Cyclone wavelet path), Pluto+ TX confirmed |
| H-QAM1 Constellation profiling | 🔴 Not started | Needs H1 (ray casting), Pluto+ loopback |
| H-QAM2 Multi-layer packing | 🔴 Not started | Needs H-QAM1 ceiling measurement |
| H3-DPC1 Costa/THP precoding | 🔴 Not started | Needs G4 PINN (channel matrix H) |
| H3-DPC2 Room-as-codec null | 🔴 Not started | Needs H3-DPC1 |
| H-FRAC1 GPU IDWT fractal shader | 🔴 Not started | Needs A-WOFDM proven; Audio backend first |
| K1 rpitx W-OFDM PoC | 🔴 Not started | Needs A-WOFDM, Toto Pi node |
| K2 Impulse ranging | 🔴 Not started | Needs K1, WRF-GS scene |
| K3 Fractal mesh | 🔴 Not started | Needs K1, two Toto nodes |
