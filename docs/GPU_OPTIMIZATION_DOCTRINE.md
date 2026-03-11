# GPU Optimization Doctrine: Wave64 Latency Hiding on RDNA2

**Hardware Target:** AMD Radeon RX 6700 XT (RDNA2 Architecture)
**Proven Baseline:** Wave64 + 256-byte alignment (33.8ms for 10k particles, 1024×1024 viewport)
**Performance Anti-Pattern:** Wave32 mode (4.0x slower—confirmed via empirical testing)

---

## Three Pillars of Optimized Wave Scheduling

### Pillar 1: Register Pressure vs. Occupancy

**The Reality:** Your RX 6700 XT's WGP has a finite pool of Vector General-Purpose Registers (VGPRs). The GPU scheduler's job is to pack as many in-flight Wave64s onto the silicon as possible.

**The Mechanism:**
- When a Wave64 stalls waiting for VRAM (high-latency fetch), the scheduler swaps to another Wave64 in the VGPR pool to execute ALU math
- This is called **Latency Hiding**—the GPU hides the 200+ cycle VRAM latency by doing useful work on other waves
- If occupancy is too low, there's no backup wave to swap to, and the GPU sits idle

**The Trap:**
```wgsl
// BAD: Massive variable scope = high register pressure
fn bad_example() {
    var particle_azimuth: f32;
    var particle_elevation: f32;
    var particle_frequency: f32;
    var accumulated_color_r: f32;
    var accumulated_color_g: f32;
    var accumulated_color_b: f32;
    var accumulated_color_a: f32;
    var distance_squared: f32;
    var gaussian_weight: f32;
    var intensity: f32;
    // ... 10+ more variables declared upfront
    // Compiler allocates VGPRs for ALL of these immediately
    // Occupancy plummets
}
```

**The Fix:**
```wgsl
// GOOD: Reuse variables, tight scope
fn good_example() {
    var accumulated = vec4<f32>(0.0);  // Single vec4 instead of 7 f32s

    for (var i: u32 = 0u; i < particle_count; i++) {
        let particle = load_particle(i);  // Load into temporary registers
        let gaussian = compute_gaussian(particle);  // Reuse local scope
        let color = tonemap_frequency(particle.frequency);
        accumulated += vec4<f32>(color * gaussian, gaussian);
        // Variables exit scope immediately, registers freed
    }
    return accumulated;
}
```

**Action Items:**
- [ ] Minimize variable declarations at function scope
- [ ] Declare variables as close to use as possible
- [ ] Prefer `let` (immutable, tighter scope) over `var`
- [ ] Avoid unrolling loops if it causes spilling (keep loops compact)
- [ ] Profile with `GSPLAT_ALIGNMENT=256` baseline—any register spilling immediately visible as slowdown

---

### Pillar 2: Slaying Thread Divergence

**The Reality:** In a Wave64, all 64 threads execute the exact same instruction at the same time (SIMD). They share a single Program Counter.

**The Trap:**
```wgsl
// BAD: Divergent branching
fn bad_filtering(particle: Particle) -> vec4<f32> {
    if (particle.threat_level > 0.5) {
        return tonemap_red(particle.frequency);      // Path A: 32 threads
    } else {
        return tonemap_blue(particle.frequency);     // Path B: 32 threads
    }
    // Wave64 executes BOTH paths sequentially
    // Path A: 32 ALUs work, 32 ALUs idle (masked off)
    // Path B: 32 ALUs work, 32 ALUs idle (masked off)
    // TOTAL: 2x slowdown due to serialization
}
```

**Why This Hurts:**
- The scheduler cannot start the `else` block until ALL threads complete the `if` block
- Half the compute units sit idle during each branch
- A divergent if/else inside a loop repeats this penalty every iteration

**The Fix: Mathematical Masking**
```wgsl
// GOOD: No branching, pure math
fn good_filtering(particle: Particle) -> vec4<f32> {
    let threat_mask = f32(particle.threat_level > 0.5);  // 1.0 or 0.0
    let safe_mask = 1.0 - threat_mask;

    let red_color = tonemap_red(particle.frequency);
    let blue_color = tonemap_blue(particle.frequency);

    // No branching: both paths computed, one zeroed out
    return red_color * threat_mask + blue_color * safe_mask;
    // All 64 threads execute identical instructions
    // 100% ALU utilization
}
```

**Another Example: Clipping**
```wgsl
// BAD
if (value > max_threshold) {
    value = max_threshold;
}

// GOOD
value = min(value, max_threshold);
```

**Action Items:**
- [ ] Scan all shaders for `if`/`else` statements
- [ ] Replace with mathematical masking: `val * f32(condition)` or `select()` WGSL function
- [ ] Ensure all code paths (true and false) produce results; multiply one by 0.0
- [ ] Keep workgroup execution completely uniform—no thread divergence
- [ ] Test with a divergence-heavy workload to see the delta (expect 2x slowdown if present)

---

### Pillar 3: Subgroup Operations (The Bleeding Edge)

**The Reality:** Modern GPUs (RDNA2 included) support **Subgroup Operations**—direct register-to-register communication between threads in a wave, bypassing VRAM and L1 cache entirely.

**Why This Matters:**
- Normal reduction: 64 threads write to shared memory → VRAM latency (200+ cycles)
- Subgroup reduction: 64 threads talk via ALU registers → single clock cycle

**WGSL Subgroup API** (requires `wgpu::Features::SUBGROUP`):
```wgsl
// Find max frequency in the wave without touching VRAM
let max_freq_in_wave = subgroupMax(particle.frequency);

// Sum all frequencies (e.g., for normalization)
let sum_freq = subgroupAdd(particle.frequency);

// Broadcast value from thread 0 to all 64 threads
let reference_color = subgroupBroadcast(color, 0u);

// Exclusive scan (prefix sum)
let cumulative = subgroupExclusiveAdd(weight);
```

**When to Use:**
- Reducing large arrays of values (e.g., find peak frequency from 10k particles)
- Broadcasting reference data (e.g., grid bounds) to all threads
- Per-wave statistics (e.g., average intensity for tone-mapping)

**Example: Optimal Tone-Mapping**
```wgsl
// OLD (VRAM-based): Write all intensities to shared array, find max
// NEW (Subgroup-based): Direct register communication
var intensity: f32 = compute_intensity();

// This single instruction broadcasts max to all 64 threads
let max_intensity_in_wave = subgroupMax(intensity);

// Normalize without hitting VRAM
let normalized = intensity / max(max_intensity_in_wave, 0.001);

// All 64 threads now know the wave's peak—can tonemap consistently
return tonemap_frequency(particle.frequency, normalized);
```

**Action Items:**
- [ ] Enable `wgpu::Features::SUBGROUP` in device initialization (already done in gaussian_splat_bench.rs)
- [ ] Profile reduction/broadcast operations to identify VRAM bottlenecks
- [ ] Replace shared memory reductions with `subgroupAdd()`, `subgroupMax()`, etc.
- [ ] Ensure subgroup operations are inside uniform control flow (no divergence allowed)

---

## Configuration Checklist for All WGSL Shaders

### Memory & Alignment
- [ ] **Workgroup Size:** Multiple of 64 (e.g., 64x1, 32x2, 16x4)
- [ ] **Buffer Alignment:** 256-byte (non-negotiable for RDNA2)
- [ ] **Buffer Layout:** Struct-of-Arrays (SoA), not Array-of-Structs
- [ ] **Timestamp Queries:** Enable `TIMESTAMP_QUERY` for profiling

### Register Pressure
- [ ] **Variable Scope:** Minimal, tight scoping with `let` preferred over `var`
- [ ] **Loop Unrolling:** Disabled (keep compiled code compact)
- [ ] **Occupancy Target:** Aim for 8+ waves per WGP (measured via profiling)

### Thread Divergence
- [ ] **Branching Audit:** Zero divergent if/else in inner loops
- [ ] **Mathematical Masking:** All conditionals use `val * f32(cond)` or `select()`
- [ ] **Uniform Execution:** All threads execute identical instruction stream

### Subgroup Optimization
- [ ] **Reduction Operations:** Use `subgroupAdd()`, `subgroupMax()` for cross-thread communication
- [ ] **Broadcasting:** Use `subgroupBroadcast()` for reference values
- [ ] **Feature Gate:** Only in code paths where SUBGROUP is enabled

---

## Performance Baseline (Locked In)

```
Configuration                   GPU Time    Occupancy   Variant
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Wave64 + 256-byte alignment     33.8 ms     ✓✓✓✓✓      BASELINE ✓
Wave64 + 128-byte alignment     35.6 ms     ✓✓✓✓       +5.3%
Wave32 + 256-byte alignment    136.9 ms     ✗✗✗        -4.0x (NEVER USE)
Wave32 + 128-byte alignment    142.1 ms     ✗✗✗        -4.2x (NEVER USE)
```

**Why Wave32 Fails:**
- Register pressure remains constant (Wave32 has same VGPR pool)
- But only 32 threads per wave instead of 64
- Scheduler cannot fill GPU with enough work-in-flight
- Memory stalls cause complete idleness (no backup wave to swap to)

---

## Directive for Future Coding Agents

**When writing or optimizing shaders for `dispatch_kernel.wgsl`, `gaussian_splatting.wgsl`, or any TimeGNN clustering kernels:**

> "Optimize for Wave64 Occupancy and Subgroup mechanics. Eliminate all divergent if/else branches using mathematical masking (e.g., `val * f32(condition)`). Minimize variable scope to reduce VGPR pressure, and ensure workgroup sizes are multiples of 64 to perfectly saturate the AMD scheduler without leaving ALUs idle. Profile with `GSPLAT_ALIGNMENT=256` and `GSPLAT_WAVE=64` as the mandatory baseline."

---

## References

- **RDNA2 Architecture:** https://en.wikichip.org/wiki/amd/rdna_2
- **WGSL Spec:** https://www.w3.org/TR/WGSL/
- **wgpu Features:** https://docs.rs/wgpu/latest/wgpu/struct.Features.html
- **Hardware Profiling Results:** See `examples/gaussian_splat_bench.rs` (run with various `GSPLAT_WAVE` and `GSPLAT_ALIGNMENT` env vars)

---

## Empirical Evidence

**Benchmark Date:** 2026-03-08
**Hardware:** AMD Radeon RX 6700 XT (12GB VRAM, Vulkan backend)
**Test:** 10,000 particles, 1024×1024 compute grid
**Verdict:** Wave64 + 256-byte alignment is the production-grade optimum. Do not deviate.

