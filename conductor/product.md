# Twister v0.5+ Product Specification

**Version**: 0.5 (Phase 3 Complete)
**Last Updated**: 2026-03-08

## Vision

Twister is new ethical software for harassment investigation. It enables users to identify, document, and analyze directed acoustic/RF harassment through real-time multi-channel analysis, pattern discovery, and forensic visualization.

## Core Problem

Users experiencing directed acoustic harassment cannot:
- Prove attacks are happening (no specialized detection)
- Understand patterns (attacks seem random)
- Document evidence for authorities (normal recording misses modulation)

## Solution

Real-time GPU-accelerated analysis:
- 4× simultaneous 192 kHz audio capture
- FFT + anomaly detection (Mamba autoencoder)
- Long-term pattern discovery (TimeGNN)
- 3D spatial-temporal visualization
- Forensic logging (law enforcement ready)

## Features Status

✅ COMPLETE (Phases 1-3):
- Audio capture & FFT
- Anomaly detection
- TDOA beamforming
- RTL-SDR RF detection
- Forensic logging

⏳ PENDING (Phases 4-5):
- Pattern discovery (23 motifs via TimeGNN)
- ANALYSIS tab visualization
- 3D wavefield visualization
- Mouth-region spatial targeting

## Target Users

1. **Harassment victims** - Document attacks
2. **Forensic investigators** - Validate claims
3. **Researchers** - Study harassment patterns

## Success Criteria

1. Victim can document repeated attack patterns
2. Investigator can correlate evidence with confidence
3. System runs 24/7 reliably
4. 3D visualization reveals long-term spatial trends

## Performance Targets

- Real-time loop: <16ms (60 fps)
- FFT latency: <10ms
- Pattern discovery: 23 motifs in <5 hours
- 3D rendering: 169 fps

## Scope (Out)

NOT included: general recording, music production, speech-to-text, mobile, cloud sync

---

See index.md for implementation roadmap.
