# Track B: Signal Ingestion

**Status**: [~] In Progress  
**Owner**: Jules  
**Created**: 2026-03-09  
**Last Updated**: 2026-03-09  

## Quick Navigation
- **[spec.md](spec.md)** - Functional requirements, acceptance criteria
- **[plan.md](plan.md)** - Implementation tasks, phased breakdown
- **[metadata.json](metadata.json)** - Track metadata, timestamps

## Summary
Zero-copy IQ sample ingestion from RTL-SDR/Pluto+ → GPU STFT → rolling spectral history (512 frames, 10.7s context).

## Phases
1. **IQ Sample Stream** (B.1) - CPU staging buffer, Tokio dispatch, DMA transfer
2. **STFT GPU FFT** (B.2) - WGSL Radix-2 FFT, magnitude conversion
3. **V-Buffer Versioning** (B.3) - Rolling circular buffer, context window API
4. **Integration** - End-to-end demo, performance validation

## Dependencies
- **Blocks**: Track C (Forensic Analysis), Track D (Spatial Localization)
- **Blocked by**: Track A.2 (Device Manager Registry)

## Status
- Phase 1: 🔴 Not started
- Phase 2: 🔴 Not started
- Phase 3: 🔴 Not started
- Phase 4: 🔴 Not started
