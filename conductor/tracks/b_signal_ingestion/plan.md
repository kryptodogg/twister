# Implementation Plan: Signal Ingestion (Audio → GPU)

## Phase 1: IQ Sample Stream (B.1)
- [ ] **Task 1.1: Create IQ Staging Buffer**
    - [ ] Create `src/hardware_io/iq_staging_buffer.rs`
    - [ ] Implement `IqStagingBuffer` struct with host-visible mapping
    - [ ] **Write Tests:** `examples/test_iq_staging_buffer.rs`
    - [ ] **Implement:** Buffer allocation, write, and read operations

- [ ] **Task 1.2: Tokio Dispatch Loop**
    - [ ] Create `src/dispatch/iq_dispatch.rs`
    - [ ] Implement async loop: `device.read_sync()` → staging buffer
    - [ ] **Write Tests:** `examples/test_iq_dispatch_loop.rs`
    - [ ] **Implement:** Backpressure handling, error recovery

- [ ] **Task 1.3: DMA Transfer to GPU**
    - [ ] Create `src/hardware_io/dma_transfer.rs`
    - [ ] Implement `dma_copy_to_gpu()` with zero intermediate conversion
    - [ ] **Write Tests:** `examples/test_dma_iq_transfer.rs`
    - [ ] **Implement:** Staging → VRAM transfer (wgpu buffer copy)

- [ ] **Task: Conductor - User Manual Verification 'Phase 1' (Protocol in workflow.md)**

## Phase 2: STFT GPU FFT (B.2)
- [ ] **Task 2.1: WGSL FFT Kernel**
    - [ ] Create `src/visualization/shaders/stft_iq.wgsl`
    - [ ] Implement Radix-2 Cooley-Tukey FFT (2048 → 512 bins)
    - [ ] **Write Tests:** `examples/test_stft_fft_correctness.rs`
    - [ ] **Implement:** Workgroup shared memory, butterfly operations

- [ ] **Task 2.2: Magnitude Conversion Shader**
    - [ ] Extend `stft_iq.wgsl` with magnitude compute pass
    - [ ] Implement log-scale compression: `log2(1.0 + magnitude)`
    - [ ] **Write Tests:** `examples/test_stft_magnitude.rs`
    - [ ] **Implement:** Normalization to [0.0, 1.0] range

- [ ] **Task 2.3: FFT Pipeline Integration**
    - [ ] Create `src/visualization/stft_pipeline.rs`
    - [ ] Wire dispatch: IQ buffer → FFT → magnitude output
    - [ ] **Write Tests:** `examples/test_stft_pipeline.rs`
    - [ ] **Implement:** Pipeline lifecycle (init, dispatch, cleanup)

- [ ] **Task: Conductor - User Manual Verification 'Phase 2' (Protocol in workflow.md)**

## Phase 3: V-Buffer Versioning (B.3)
- [ ] **Task 3.1: GpuVBuffer Rolling Index**
    - [ ] Extend `src/vbuffer.rs::GpuVBuffer`
    - [ ] Add `version: u64` atomic counter
    - [ ] Implement `push_frame()` with modulo indexing
    - [ ] **Write Tests:** `examples/test_vbuffer_rolling_index.rs`

- [ ] **Task 3.2: Spectral History Buffer**
    - [ ] Allocate GPU storage: 512 frames × 512 bins × 4 bytes
    - [ ] Implement CPU readback mapping for forensic access
    - [ ] **Write Tests:** `examples/test_vbuffer_history_readback.rs`
    - [ ] **Implement:** Zero-copy CPU read pointer

- [ ] **Task 3.3: Context Window API**
    - [ ] Add `get_context_window(n_frames: usize)` method
    - [ ] Handle wraparound correctly (circular buffer semantics)
    - [ ] **Write Tests:** `examples/test_vbuffer_context_window.rs`
    - [ ] **Implement:** Efficient slice extraction

- [ ] **Task: Conductor - User Manual Verification 'Phase 3' (Protocol in workflow.md)**

## Phase 4: Integration & Performance Validation
- [ ] **Task 4.1: End-to-End Pipeline Test**
    - [ ] Create `examples/full_signal_ingestion_demo.rs`
    - [ ] Wire: RTL-SDR → staging → DMA → FFT → vbuffer
    - [ ] **Write Tests:** Full pipeline smoke test (30s runtime)
    - [ ] **Implement:** Performance metrics logging

- [ ] **Task 4.2: Performance Benchmarking**
    - [ ] Create `benches/signal_ingestion.rs`
    - [ ] Benchmark: FFT latency, DMA throughput, vbuffer push
    - [ ] **Targets:** <200μs end-to-end, >90% GPU util
    - [ ] **Implement:** Criterion-based benchmarks

- [ ] **Task 4.3: Edge Case Testing**
    - [ ] Test: Device disconnect/reconnect
    - [ ] Test: Buffer overflow handling
    - [ ] Test: Empty input (no signal)
    - [ ] **Write Tests:** `examples/test_signal_ingestion_edge_cases.rs`

- [ ] **Task: Conductor - User Manual Verification 'Phase 4' (Protocol in workflow.md)**

## Deliverables
1. `src/hardware_io/iq_staging_buffer.rs` - CPU staging buffer
2. `src/dispatch/iq_dispatch.rs` - Tokio streaming loop
3. `src/visualization/shaders/stft_iq.wgsl` - FFT + magnitude shaders
4. `src/visualization/stft_pipeline.rs` - GPU pipeline orchestration
5. `src/vbuffer.rs` (extended) - Rolling circular buffer
6. 10+ integration tests in `examples/`
7. Performance benchmarks in `benches/`
