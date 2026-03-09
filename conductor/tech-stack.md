# Tech Stack & Architecture Decisions

**Project**: Twister v0.5+
**Last Updated**: 2026-03-08

## Primary Languages & Frameworks

### Rust (Core Application)
- **Edition**: 2024
- **Use**: Real-time dispatch loop, GPU orchestration, forensic logging
- **Rationale**: Memory safety, zero-cost abstractions, GPU interop

### GPU Compute (wgpu + WGSL)
- **Framework**: wgpu (WebGPU Rust bindings)
- **Shaders**: WGSL (Web Shader Language)
- **Target**: AMD Radeon RX 6700 XT (RDNA2, Vulkan backend)
- **Architecture**: Wave64 occupancy optimization, subgroup operations
- **Rationale**: Cross-platform GPU compute, safety, performance

## ML Frameworks

### Burn (Primary ML Backend)
- **Version**: 0.21.0-pre.2 (from git main)
- **Use**: Neural network layers (Linear, Conv1d, BatchNorm, LayerNorm)
- **Backends**: NdArray (CPU) + potential CUDA/Wgpu
- **Rationale**: Generic backend trait, Rust-native, type-safe
- **Modules**: 
  - ModularFeatureExtractor (Task 1)
  - TimeGNN (Phase 4)
  - PointMamba (Phase 5)

### Candle (Legacy/Optional)
- **Version**: 0.3+
- **Use**: MambaAutoencoder (existing real-time anomaly detector)
- **Status**: Works but being superseded by Burn-based components
- **Rationale**: HuggingFace alignment, but transitioning to Burn

### candle-transformers (External Models)
- **Use**: Load facebook/wav2vec2-base-960h frozen embeddings
- **Status**: Frozen weights only (no gradient computation)
- **Version**: From HuggingFace hub downloads

## Audio & Signal Processing

### cpal 0.17.3
- **Use**: Multi-channel audio I/O (4 simultaneous devices)
- **Sample Rate**: 192 kHz input, 48 kHz output
- **API Breaking Changes**: SampleRate now struct (not u32)

### rfft2 (FFT Library)
- **Use**: Real FFT computation (512-bin spectrum)
- **Window**: Hann window
- **Output**: Magnitude spectrum (log scale for UI)

### Time-Difference-of-Arrival (Custom)
- **Algorithm**: Cross-correlation of mic pairs
- **Output**: Azimuth estimation (2D)
- **Future**: Elevation estimation (Phase 4 Fix #3)

## Hardware Integration

### RTL-SDR (Software Receiver)
- **Device**: RTL2838UHIDIR (Blog V4)
- **Frequency**: 2.4 GHz (modulation detection)
- **Library**: rtlsdr-sys (FFI bindings)
- **Data**: Magnitude spectrum, 512 bins

### ANC (Active Noise Cancellation)
- **Calibration**: 8192-bin lookup table (1 Hz - 12.288 MHz)
- **Filter**: LMS (Least Mean Squares) adaptive
- **Output**: Synthesized counter-phase waveform

## UI Framework

### Slint 1.15.1
- **Language**: .slint (declarative UI)
- **Reactivity**: Property binding + callbacks
- **Components**: Oscilloscope, waterfall, spectrum bars, ANALYSIS tab
- **Performance**: Uncapped framerate (30-60 fps typical)
- **API Alignment**: Fixed in latest 1.15.1

### SVG Path Generation (Visualization)
- **Method**: String building (M/L/Z commands)
- **Use**: Spectrum lines, waveform display
- **Reactive**: Updated every ~100ms from Rust state

## Databases & Logging

### Neo4j (Forensic Correlation)
- **Use**: Graph of event relationships (temporal, spectral, spatial)
- **Schema**: Event nodes, Source nodes, Cluster nodes
- **Queries**: Find events with similar signatures

### SQLite (Audit Log)
- **Use**: Session audit trail, quality assurance
- **Retention**: Indefinite (backups handled externally)

### JSONL (Forensic Event Stream)
- **Format**: One JSON object per line
- **Fields**: timestamp_us, anomaly_score, frequency_hz, equipment_id
- **Location**: @databases/forensic_logs/events.jsonl

### HDF5 (Offline Training Corpus)
- **Use**: Store multimodal feature vectors + events
- **Layout**: [N_events, feature_dim], timestamps, labels
- **Generated**: Phase 4 B.1 (wav2vec2 integration)

## Dependency Versions

| Crate | Version | Use |
|-------|---------|-----|
| burn | 0.21.0-pre.2 | ML (encoder/decoder) |
| candle-core | 0.3+ | MambaAutoencoder |
| wgpu | 0.19+ | GPU compute |
| cpal | 0.17.3 | Audio I/O |
| slint | 1.15.1 | UI |
| tokio | 1.35+ | Async runtime |
| serde_json | 1.0+ | Serialization |
| ndarray | 0.15+ | Array operations |

## GPU Optimization Doctrine

**Target**: RX 6700 XT (RDNA2)
**Baseline**: Wave64 + 256-byte memory alignment
**Performance**: 33.8ms for 10k particles, 1024×1024 viewport

### Key Principles
1. **Wave64 occupancy**: Minimize VGPR pressure, enable latency hiding
2. **Eliminate divergence**: Use mathematical masking (no if/else in inner loops)
3. **Subgroup ops**: Use broadcast/reduce instead of shared memory
4. **Memory alignment**: 256-byte (non-negotiable)
5. **Workgroup size**: Multiple of 64 (32x2, 16x4, 64x1)

### Forbidden
❌ Wave32 (4.0x slower on RDNA2)
❌ Divergent branching in inner loops
❌ Unaligned memory access
❌ Shared memory where subgroup ops work

## Development Tools

### Build System
- **Cargo**: Package manager, build orchestration
- **build.rs**: Copies RTL-SDR DLLs at compile time
- **Target**: x64 Windows (MSVC toolchain)

### Testing
- **Framework**: cargo test (Rust standard)
- **Coverage**: Unit + integration tests
- **Minimum**: 10 tests per feature before UI wiring

### Debugging
- **Console**: eprintln! (redirected to GUI console in Phase 2 Fix #2)
- **Logging**: Forensic logs (structured, queryable)
- **Profiling**: wgpu frame timing, CPU util via std::time

### Version Control
- **Repository**: Git (GitHub)
- **Branching**: feature/{phase}-{component}, main (stable)
- **Workflow**: TDD + feature branches + PR review (see workflow.md)

## Architectural Decisions

### Why Burn + wgpu instead of PyTorch?
- Rust eliminates memory safety bugs
- Unified language (Rust everywhere)
- wgpu provides cross-platform GPU compute
- Faster iteration on GPU-tight loops

### Why Slint instead of Qt/Imgui?
- Declarative (easier to extend)
- Reactive bindings (automatic updates)
- Lightweight (no heavy C++ runtime)
- Rust integration (type-safe callbacks)

### Why Neo4j instead of PostgreSQL?
- Graph queries (event relationships)
- Flexibility (schema-less)
- Pattern matching (find similar harassment signatures)

### Why Burn v0.21-pre.2?
- Latest API (squeeze, mean_dim changes)
- Pre-release for early adopter benefits
- Opportunity to influence API design
- Production stability acceptable (used in tests)

---

## Framework Transitions

### Migration Path: Candle → Burn
- MambaAutoencoder (Candle) will be superseded by Burn-based anomaly detector
- Timeline: After Phase 4 (low priority)
- Compatibility: Can coexist via adapter layer if needed

### API Compatibility
- Burn API changes (squeeze_dim, unsqueeze_dim) = handled in code
- Slint API changes = resolved in latest 1.15.1
- cpal SampleRate breaking change = wrapped in helper functions

---

See product.md for feature roadmap and requirements.
