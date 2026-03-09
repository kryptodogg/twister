# Implementation Plan: Circular GPU Buffer & Phase State Persistence

## Phase 1: Persistent GPU State
- [ ] **Task: Phase 1.1 - Create GpuState Struct**
    - [ ] Create `GpuState` struct for Phase, Sweep, LCG, and PDM coefficients.
    - [ ] **Write Tests:** Verify state initialization and consistency.
    - [ ] **Implement:** Define the struct in `gpu.rs`.
- [ ] **Task: Phase 1.2 - Allocate STORAGE Buffer**
    - [ ] **Write Tests:** Ensure buffer allocation and mapping are successful.
    - [ ] **Implement:** Allocate a `wgpu::Buffer` with `STORAGE` usage in `gpu.rs`.
- [ ] **Task: Phase 1.3 - Update Synthesis Shader**
    - [ ] **Write Tests:** Verify phase continuity across multiple frames in the shader.
    - [ ] **Implement:** Update `SYNTHESIS_SHADER` to use the new storage binding for atomic phase accumulation.
- [ ] **Task: Conductor - User Manual Verification 'Phase 1' (Protocol in workflow.md)**

## Phase 2: Virtual Ring Buffer (Waterfall)
- [ ] **Task: Phase 2.1 - Remove Shift Pipeline**
    - [ ] **Write Tests:** Ensure the waterfall still renders without the shift pass.
    - [ ] **Implement:** Remove `scroll_and_insert` pipeline and shader code from `waterfall.rs`.
- [ ] **Task: Phase 2.2 - Add Row Offset**
    - [ ] **Write Tests:** Verify `WaterfallParams` correctly tracks the rolling offset.
    - [ ] **Implement:** Add `row_offset: u32` to `WaterfallParams`.
- [ ] **Task: Phase 2.3 - Update Colormap Shader**
    - [ ] **Write Tests:** Ensure physical row mapping matches expected visual output.
    - [ ] **Implement:** Update `colormap_all` shader for physical row mapping.
- [ ] **Task: Conductor - User Manual Verification 'Phase 2' (Protocol in workflow.md)**

## Phase 3: Parallel Reductions & Bispectrum Optimization
- [ ] **Task: Phase 3.1 - Rewrite Downsample Shader**
    - [ ] **Write Tests:** Benchmark the new tree-based reduction against the previous linear loop.
    - [ ] **Implement:** Use `@workgroup_size(256)` and shared memory for tree-based reduction.
- [ ] **Task: Phase 3.2 - Optimize Bispectrum Kernels**
    - [ ] **Write Tests:** Profile bispectrum calculation for <200μs latency target.
    - [ ] **Implement:** Parallelize bispectrum kernels for high GPU utilization.
- [ ] **Task: Phase 3.3 - Add Windowing Pre-pass**
    - [ ] **Write Tests:** Verify side-lobe suppression on raw RTL-SDR data.
    - [ ] **Implement:** Write and integrate a new Hann/Hamming windowing compute kernel.
- [ ] **Task: Conductor - User Manual Verification 'Phase 3' (Protocol in workflow.md)**

## Phase 4: Desperado Integration
- [ ] **Task: Phase 4.1 - Add Desperado Dependency**
    - [ ] **Write Tests:** Verify dependency resolves and compiles.
    - [ ] **Implement:** Add `desperado = "0.3.0"` to `Cargo.toml`.
- [ ] **Task: Phase 4.2 - Integrate Desperado Hardware Layer**
    - [ ] **Write Tests:** Validate communication with Pluto+ and RTL-SDR using the new crate.
    - [ ] **Implement:** Use `desperado` for low-level device control and buffer management.
- [ ] **Task: Conductor - User Manual Verification 'Phase 4' (Protocol in workflow.md)**
