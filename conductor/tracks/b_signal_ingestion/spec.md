# Track Specification: Signal Ingestion (Audio → GPU)

## Overview
Implement zero-copy IQ sample ingestion pipeline from RTL-SDR/Pluto+ devices to GPU for STFT processing and rolling spectral history. This track establishes the foundational data path for all downstream forensic analysis.

## Functional Requirements

### B.1: IQ Sample Stream from Devices
1. **Zero-Copy Buffer Architecture**:
   - CPU-side staging buffer for raw `[u8; 2]` IQ samples
   - DMA transfer to GPU VRAM without f32 conversion on CPU
   - Tokio-based dispatch loop for continuous streaming

2. **Device Integration**:
   - RTL-SDR via `rtlsdr-sys` FFI
   - Pluto+ via `desperado` crate
   - Unified `RadioDevice` enum for device abstraction

3. **Performance Targets**:
   - Sample rate: 2.4 MS/s (RTL-SDR)
   - CPU utilization: <10% during ingestion
   - Zero heap allocations in hot path

### B.2: STFT (GPU FFT on IQ Data)
1. **WGSL FFT Implementation**:
   - Radix-2 FFT: [2048] complex → [512] bins
   - In-place computation for memory efficiency
   - Workgroup size: 256 threads (Wave64 optimized)

2. **Spectral Magnitude Output**:
   - Complex → magnitude conversion (log scale)
   - Output format: f32 normalized [0.0, 1.0]
   - Compatible with existing waterfall display

3. **Performance Targets**:
   - FFT latency: <100μs per frame
   - GPU utilization: >90%

### B.3: V-Buffer Versioning
1. **Rolling Circular Buffer**:
   - Depth: 512 frames (10.7s context window at 48 fps)
   - Index calculation: `version % DEPTH`
   - Atomic version counter for thread safety

2. **GPU Memory Layout**:
   - Storage buffer for spectral history
   - CPU-visible mapping for forensic logging
   - Zero-copy readback for analysis

3. **Performance Targets**:
   - Frame push: <1μs (index calculation only)
   - Memory footprint: 512 × 512 × 4 bytes = 1 MB

## Acceptance Criteria
- [ ] **B.1**: RTL-SDR streams IQ samples to GPU staging buffer (zero f32 conversion)
- [ ] **B.2**: WGSL FFT produces correct spectral magnitude (verified against reference)
- [ ] **B.3**: V-buffer maintains 512-frame rolling history without artifacts
- [ ] **Performance**: End-to-end latency <200μs (device → GPU → spectral history)
- [ ] **Tests**: 10+ integration tests covering edge cases and performance benchmarks

## Dependencies
- **Blocks**: Track C (Forensic Analysis), Track D (Spatial Localization)
- **Blocked by**: Track A.2 (Device Manager Registry)
- **Parallel-safe**: Track H (HID), Track E (Dorothy UI)

## Out of Scope
- Audio device ingestion (separate track)
- Visual feature extraction (Task 1)
- Pattern discovery algorithms (Track C)
