# Track Specification: Circular GPU Buffer & Phase State Persistence

## Overview
Refactor the GPU synthesis and waterfall engines to replace the O(N*M) memory shift with a Virtual Ring Buffer and persistent state tracking. This will maximize GPU utilization (target: 92%) and optimize the bispectrum analysis pipeline. Additionally, integrate the `desperado` crate to support low-level hardware control for Pluto+ and RTL-SDR.

## Functional Requirements
1.  **Virtual Ring Buffer (Waterfall):** 
    - Completely replace the current O(N*M) shift operation with a rolling row index offset (`actual_row = (requested_row + offset) % n_rows`).
    - Update `WaterfallParams` to track the current offset.
2.  **Persistent State Buffer (GPU Synthesis):**
    - Move **Phase & LCG State**, **Peak History**, and **PDM Coefficients** to a persistent `STORAGE` Buffer.
    - Implement an atomic phase accumulator in the WGSL shader.
3.  **Full Pipeline Reduction:**
    - Optimize **all spectral analysis passes**, including bispectrum, using tree-based parallel reductions in workgroup shared memory.
4.  **Hardware & Tech Stack:**
    - Add `desperado = "0.3.0"` to `Cargo.toml`.
    - Integrate `desperado` for low-level device control and optimized buffer management for Pluto+ and RTL-SDR.
5.  **Windowing Pre-pass:**
    - Apply Hann/Hamming windowing before the FFT to suppress side-lobes.

## Acceptance Criteria
- [ ] Memory shift is completely eliminated from the waterfall display.
- [ ] Synthesis phase accumulation is strictly GPU-side for sweep continuity.
- [ ] Bispectrum analysis latency is reduced from ~2.1ms to <200μs.
- [ ] GPU utilization is verified at >90% during full pipeline operation.
- [ ] `desperado` is successfully integrated and validated for Pluto+ / RTL-SDR communication.
