# Project Synesthesia — Physics, Haptics & Material Addendum
## Missing Milestones from Older Architecture Documents

**Addendum to**: `ROADMAP.md`  
**Status**: Fold these milestones into existing tracks. No new crate structure.
**Principle**: Ideas survive; the sprawling crate layout does not.

---

## What Was Missing From the Base Roadmap

Four concept areas were present in earlier design documents but absent from ROADMAP.md:

1. **Particle physics simulation** (SPH-PBD) — RF particles need kinetic animus, not
   just visual position. Frozen data points are not a living field.
2. **RF-BSDF material model** — physically correct RF scattering to replace the
   implied optical-PBR defaults. Complex Fresnel, RF-GGX, Double-Debye wetness.
3. **600Hz haptic sub-step** — tactile feedback decoupled from the 60Hz visual frame.
4. **EMERALD CITY phase coherence** — the current color mapper is frequency→hue. The
   full model also encodes phase coherence as a scalar that drives the lighting engine,
   enabling destructive interference to show as darkness rather than just wrong color.

These are all additions to existing tracks. The 7 RF-BSDF extension tracks are listed
at the end as future phases, beyond Track I.

---

## Additions to Track G — WRF-GS Scene

### G2-RDNA2: Wave64 Compute Mandate (amend G2)

All particle and Gaussian evaluation shaders must use `@workgroup_size(64, 1, 1)`.
This is a hard requirement for the RX 6700 XT, not a preference. RDNA 2 executes
wavefronts of exactly 64 threads. A workgroup of 32 leaves half the ALUs idle.
A workgroup of 128 causes two sequential wavefronts with cross-wavefront overhead.

**Hard rules for all shaders in the visualization pipeline:**
- `@workgroup_size(64, 1, 1)` always — never 32, never 128
- Structure of Arrays (SoA) layout for any buffer holding > 10k elements —
  keeps each field's memory contiguous for coalesced reads
- Any struct that crosses the CPU/GPU boundary must be sized to a multiple of
  128 bytes (one Infinity Cache line). Use named active-padding fields — never
  `[u8; N]` dummy arrays. Every padding byte carries a pre-computed heuristic
  (e.g., `doppler_shift`, `phase_velocity`, `scattering_cross_section`).
  The canonical example is `AetherParticle` (128 bytes, particle physics).
  The second canonical example is `HeterodynePayload` (128 bytes, the struct
  that carries all three sensory channels — haptic `F_tactile`, heterodyned
  `f_audio` tone frequency, and Chronos motif token — across the CPU/GPU
  boundary in a single cache line, ensuring no channel lags another).

**Acceptance**: Shader compilation log shows no "suboptimal occupancy" warnings.
GPU timestamp queries confirm the cull pass processes 1M Gaussians in ≤ 4ms.

### G-SPH: Particle Physics Sub-Track (insert between G3 and G4)

**The Breath of the Aether.** Gaussians and SPH particles are distinct objects in
the same scene. Gaussians encode EM propagation structure (where the field *is*).
SPH particles are injected into that field and governed by its pressure — they show
where energy *flows* and accumulates. A frozen particle cloud is not a living field.

**G-SPH1 — SPH density pass**
- Each particle queries its neighbors in a spatial hash grid for density summation
- Kernel: Müller 2003 poly6 for density, spiky kernel for pressure gradient
- Spatial hash: Kogge-Stone prefix scan inside a 64-thread workgroup (LDS only,
  no global atomics). Achieves O(log₂N) latency with cross-lane shuffles.
  Produces O(1) neighbor lookup for the density pass.
- **Acceptance**: 1M SPH particles, density pass ≤ 2ms GPU time.
  Neighbor lookup verified correct by checking that density in a uniform sphere
  matches the analytical result within 1%.

**G-SPH2 — PBD constraint solve**
- Incompressibility, surface tension, RF-field-driven repulsion constraints
- PBD preferred over SPH pressure projection: unconditionally stable, no
  iterative pressure solve required
- RF repulsion: particles in regions of high `phase_coherence` (from EMERALD CITY)
  experience repulsive forces. Highly coherent RF regions become "pressurized" —
  particles cluster at field boundaries, not inside them. This is the core
  visual metaphor: energy concentrates where fields interfere constructively.
- **Acceptance**: Stable at dt = 16.67ms. No explosions or tunneling.
  When a strong WiFi AP is present, particles visibly cluster at the
  constructive-interference boundary around the AP.

**G-SPH3 — Compute-to-indirect vertex pulling**
- SPH particles rendered via GPU-driven draw: compute pass culls and writes
  surviving indices to a `DrawIndirectArgs` buffer. Vertex stage pulls
  `AetherParticle` attributes directly from storage buffer using `vertex_index`.
  Zero CPU-side draw call overhead.
- **Acceptance**: `DrawIndirectArgs` populated without CPU readback.
  Particle render adds ≤ 2ms to the existing G3 frame time.

### G-RB: RF-BSDF Material Physics (insert before G4/PINN)

Standard optical PBR fails at RF wavelengths. A concrete wall looks rough at
optical scales but reflects 2.4 GHz like a mirror (roughness σ << λ_RF).
The RF-BSDF layer corrects this. All three sub-milestones feed the PINN loss
in G4 and the haptic translation in the new haptic track.

**G-RB1 — Complex Fresnel equations**
- Replace all Schlick approximations with the full complex Fresnel equations
  using complex permittivity `ε_c = ε' - jε''`
- Implemented as a WGSL compute kernel applied per Gaussian splat at material
  boundaries in the WRF-GS scene
- Material library: initial entries for dry concrete, wet concrete, glass,
  wood, metal, human body (approximate). Material parameters sourced from
  published ITU-R P.2040 tables.
- **Acceptance**: For dry concrete at 2.4 GHz (ε' ≈ 5.0, ε'' ≈ 0.17),
  reflected power matches ITU-R P.2040 Table 3 within ±1 dB.

**G-RB2 — RF-GGX microfacet distribution**
- Roughness parameter `α_RF = σ_surface / λ_RF` (surface RMS roughness
  divided by RF wavelength). The same surface is smooth at 900 MHz and rough
  at 60 GHz — `α_RF` captures this automatically.
- Transition from specular to diffuse follows Rayleigh criterion:
  `σ > λ / (8·cos θ_i)` marks the boundary
- **Acceptance**: Same concrete surface produces near-specular scattering at
  2.4 GHz (α_RF ≈ 0.01) and diffuse at 60 GHz (α_RF ≈ 0.25). Plot both
  distributions and verify the transition matches published scattering data.

**G-RB3 — Double-Debye wetness model**
- Water content shifts ε' dramatically due to Debye relaxation.
  The Double-Debye model gives frequency-dependent ε(f) for any water
  saturation level S (0 = dry, 1 = saturated)
- Effective medium approximation: ε_eff(f, S) = ε_dry + S·Δε_Debye(f)
- Runtime: a surface can be marked "wet" dynamically (e.g., when rain
  sensors or user annotation changes its saturation level)
- **Acceptance**: At f = 2.4 GHz, ε' for dry wood (≈2.0) and wet wood
  (≈20–30) produce a measurable difference in reflected power matching
  published experimental data.

---

## Additions to Track A — EMERALD CITY Phase Coherence Upgrade

### A-EC: Phase Coherence Metric (amend A1/Emerald City description)

The current color mapper does frequency → hue. That is correct but incomplete.
The full EMERALD CITY model adds a second scalar: **phase coherence** `Γ`.

```
E_total(r) = Σᵢ [ wᵢ(r) · cᵢ · G(r, rᵢ) ]

where:
  wᵢ = spatial influence kernel of path i (Gaussian footprint)
  cᵢ = complex scattering coefficient: Aᵢ · exp(jφᵢ)
  G  = free-space Green's function: exp(-jk·|r - rᵢ|) / |r - rᵢ|²

Phase coherence:
  Γ(r) = |Σᵢ E_i(r)| / Σᵢ |E_i(r)|

  Γ → 1.0: all paths arrive in phase → constructive interference → bright
  Γ → 0.0: paths cancel → destructive interference → dark (signal null)
```

**Γ maps to the lightness dimension** in the HSL color model:
- `hue`: frequency (existing logarithmic mapping, unchanged)
- `lightness`: Γ (phase coherence — bright = constructive, dark = destructive)
- `saturation`: inverse bandwidth (narrow = vivid, wideband = muted)

This means a 2.4 GHz WiFi null (standing wave zero) appears as dark violet,
not absent — it is still *there*, just canceling. The null is visible.

**Γ also feeds G-SPH2** as the RF repulsion field — highly coherent regions
repel SPH particles to field boundaries.

**Implementation note**: Γ is per-frequency, not broadband. A path length that
produces Γ ≈ 1 at 2.437 GHz (channel 6) may produce Γ ≈ 0 at 2.462 GHz
(channel 11). EMERALD CITY computes Γ per-bin across the FFT output.

**Acceptance**: Introduce a known standing wave by pointing the Pluto+ at a
metal surface at measured distance. The visualization shows a dark band at the
theoretical null distance. Verify null position matches λ/2 prediction within 5%.

**Target state**: A room in which every active RF source has been retuned —
phase-shifted or frequency-nudged by the TX pipeline — so that Γ across the
occupied bands trends toward coherence rather than cancellation. The room should
*look* like a chord: bright, saturated, organized. Not silent. Not suppressed.
Resolved. This is what Emerald City is measuring toward.

---

## New Track: Haptic Sub-System (600Hz)

**Depends on**: G-SPH2 (physics running), G-RB3 (RF-BSDF roughness available)  
**Hardware**: Voice Coil Actuators (VCAs) — exact model TBD. Start with single
VCA proof-of-concept, expand to array.

**Context**: Tactile sensation requires ≥ 300Hz update rate to feel continuous
rather than discrete taps (Nyquist for Pacinian corpuscle sensitivity peak at
200–300 Hz). The visual frame at 60 Hz cannot drive haptics directly.

### HA1 — Localized 600Hz PBD haptic solve

Only particles in the controller's bounding box are re-solved at 600 Hz.
At typical usage, this is hundreds to a few thousand particles — not 1M.
Runs on a dedicated CPU thread consuming GPU readback, not blocking the render thread.

**Budget per haptic frame: 1.67ms total**
1. GPU compute (< 0.5ms): identify proxy-region particles, PBD solve, accumulate `F_tactile`
2. Async GPU→CPU transfer (non-blocking `MAP_READ`)
3. CPU (< 0.5ms): `F_tactile` → VCA drive signal

**Async readback ring (3 buffers)**:
- While GPU writes to buffer N, CPU reads from buffer N-1, N-2 is being re-armed
- `device.poll(wgpu::Maintain::Poll)` — never `wgpu::Maintain::Wait`
- Haptic thread pinned to a dedicated CPU core, isolated from render thread

**Acceptance**: Haptic readback does not spike the 60 FPS render frame time.
VCA updates at verified 600 Hz via oscilloscope or USB audio analyzer.

### HA2 — LF/HF Bifurcation (RF-BSDF tactile texture)

Haptic output splits into two perceptual bands:

**Low-frequency (< 80 Hz) — macro pressure / bulk energy density**:
- Source: SPH pressure gradient magnitude at proxy location
- Interpretation: "how much RF energy is concentrated here"
- VCA amplitude ∝ `|∇P_SPH|`

**High-frequency (80–300 Hz) — surface texture / material identity**:
- Source: RF-GGX roughness `α_RF` and Double-Debye `ε_eff` at proxy location
- Interpretation: "what material is the RF scattering from here"
- Higher `α_RF` → higher HF vibration frequency
- This band is targeted specifically at **Pacinian corpuscles** — the mechanoreceptors
  in human fingertips sensitive to 200–300 Hz vibration. They are the biological
  texture detectors. Driving them at frequencies derived from `α_RF` means the
  user is learning material identity through the same sensory channel used to
  distinguish silk from sandpaper. A WiFi signal bouncing off concrete (α_RF ≈ 0.01
  at 2.4 GHz, smooth relative to wavelength, low HF content) feels categorically
  different from the same signal bouncing off acoustic foam (α_RF ≈ 0.3, diffuse,
  high HF content). The Pacinian corpuscles are being trained as RF material sensors.
- This is the Daredevil channel: wood, metal, glass, and wet surfaces each
  produce distinct HF signatures. The user learns to identify materials
  without visual confirmation.

**Acceptance**: With two surfaces of known different RF roughness (e.g., bare
metal vs acoustic foam at 2.4 GHz), VCA frequency spectrum shows distinct
HF content for each. Difference is perceptible in a blind A/B test.

### HA3 — Stochastic Resonance envelope

Link the RF-BSDF roughness `σ` output to a noise envelope on the haptic signal.
Stochastic resonance: adding calibrated noise to a sub-threshold signal makes
it perceptible. Applied here: rougher materials produce a noisier haptic envelope,
smoother materials produce cleaner tones. The noise level IS the texture signal.

**Acceptance**: Smooth metal surface → clean 120 Hz tone at the VCA.
Rough concrete surface → same 120 Hz tone with measurable broadband noise added.

---

## Future Phases: 7 RF-BSDF Extension Tracks

These are post-Track-I phases. They require the full A–HA pipeline to be proven
and stable before beginning. Listed here to preserve the ideas.

**Phase J1 — RF Proprioception (body schema)**
- Continuous "RF body schema" state vector: quadrant_density[4], temporal_gradient,
  occlusion_map[8×8]
- Belt/strap-mounted mmWave + Pluto+ maps how the body blocks, reflects, diffracts
- Output: slow continuous haptic pattern across torso/arm VCA array
- Not discrete events — a vestibular-style "RF balance" sense
- Test: find a router with eyes closed using only RF-body haptic gradient

**Phase J2 — Sub-Nyquist Spectral Retina**
- Compressed sensing mode: randomized LO hops + short capture windows
- Sparse recovery via IHT/OMP: "did a new emitter appear?" not full waveform fidelity
- Mamba as learned denoiser post-recovery (faster than full spectrum attention)
- Trades exact waveform for 100× faster wide-band attention

**Phase J3 — RF-Texture-Smell Chain**
- Complex permittivity classification → material label → scent profile LUT
- Petrichor for "wet porous" (ε' ≈ 25), ozone/metallic for "high σ, high reflectance"
- Hardware: Escents-class wearable scent device (magnetic smart pods)
- Optional "Olfactory Assist Mode" alongside haptic output

**Phase J4 — Ambient Backscatter Interaction Surfaces**
- Cheap RF energy-harvesting backscatter tags on objects (door frames, furniture)
- Touch/pressure changes backscatter resonance on ambient WiFi/cellular carriers
- Pluto+ sees pattern-coded scattering anomalies → map tag ID to haptic signature
- Zero-power physical controls in space — no batteries, just tags

**Phase J5 — RIS-Driven Haptic Fields**
- Wall-mounted RIS (even small 2.4/5 GHz fabric panel) with controllable phase
- Optimize RIS configuration to create sharp spatial RF gradients at user position
- Goal: "RF ridges" the user can physically walk through
- Novel direction: nobody is using RIS to craft fields whose goal is human touch

**Phase J6 — Differentiable Calibration Loop (Burn autograd)**
- Port RF-BSDF math (Fresnel, RF-GGX, Double-Debye) into Burn tensors
- Loss: predicted scattering − measured scattering from Pluto+/mmWave
- Gradient descent over ε', ε'', α_RF, water saturation S
- Brief calibration step when system is idle to keep material library tuned
  to the specific antennas, cables, and room geometry in use

**Phase J7 — Field Compass UX**
- Sub-Nyquist spectral retina (J2) scans for strongest structured RF pockets
- RF proprioception (J1) distributes direction-coded haptic gradient at waist/wrists
- "Compass needle made of vibration" pointing toward strongest RF sources
- Test protocol: RF navigation trials in Galveston, eyes closed

---

## Revised Hardware Dependency Map Additions

```
Track           Win11 only   Pluto+   Pico 2   Coral   VCA hardware
─────────────   ──────────   ──────   ──────   ─────   ────────────
A-EC                ✓
G-RDNA2             ✓
G-SPH1/2/3          ✓
G-RB1/2/3           ✓         optional
HA1                 ✓                                    ✓
HA2                 ✓                                    ✓
HA3                 ✓                                    ✓
J1                             ✓                         ✓
J2                             ✓
J3                                                       (Escents)
J4                             ✓
J5                             (RIS panel)
J6                             ✓
J7                             ✓                         ✓
```

---

## Revised Status Table Additions

| Track | Status | Blocking issue |
|-------|--------|----------------|
| A-EC Phase coherence | 🔴 Not started | Needs A1 FFT output per-bin |
| G-RDNA2 Wave64 mandate | 🔴 Not started | Enforce on all G shaders |
| G-SPH1 Density pass | 🔴 Not started | Needs G1 pipeline |
| G-SPH2 PBD solve | 🔴 Not started | Needs G-SPH1 |
| G-SPH3 Vertex pulling | 🔴 Not started | Needs G-SPH2 |
| G-RB1 Complex Fresnel | 🔴 Not started | Needs G3 (live splat data) |
| G-RB2 RF-GGX | 🔴 Not started | Needs G-RB1 |
| G-RB3 Double-Debye | 🔴 Not started | Needs G-RB1 |
| HA1 600Hz haptic solve | 🔴 Not started | Needs G-SPH2, VCA hardware |
| HA2 LF/HF bifurcation | 🔴 Not started | Needs HA1, G-RB2, G-RB3 |
| HA3 Stochastic resonance | 🔴 Not started | Needs HA2 |
| J1–J7 Extension tracks | 🔴 Post-Track-I | Needs full A–HA pipeline |

---

*This addendum supersedes the partial physics addendum (deleted).*  
*Integration order: G-RDNA2 → G-SPH1 → G-SPH2 → G-SPH3 → G-RB1 → G-RB2 → G-RB3 → HA1 → HA2 → HA3*  
*A-EC can proceed in parallel with G-SPH tracks once A1 is stable.*
