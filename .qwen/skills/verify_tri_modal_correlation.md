# Skill: verify_tri_modal_correlation

## Overview

Validates tri-modal (RF/acoustic/optical) threat correlation for SHIELD active denial. Ensures cross-modal fusion achieves required confidence thresholds and detection latency targets.

## Applicable Agents

- `tri-modal-defense-specialist`
- `mmwave-fusion-specialist`
- `shield-rf-scientist`

## Execution

```bash
# Run tri-modal correlation validation
python scripts/validate_tri_modal.py --scenario <SCENARIO_NAME> --data <TEST_DATA_DIR>

# Example: Validate Fansmitter detection with mmWave fusion
python scripts/validate_tri_modal.py --scenario fansmitter_ac1 --data test_data/ac1_fansmitter/
```

## Validation Criteria

### Pass Conditions
- Multi-modal correlation confidence: ≥ 0.8 for confirmed threats
- Detection latency RF: < 200 ms
- Detection latency acoustic: < 100 ms
- Detection latency optical: < 50 ms
- RCS reduction (cloaking): > 10 dB
- Jamming effectiveness: > 20 dB SNR reduction
- mmWave + acoustic fusion: > 90% true positive rate

### Fail Conditions
- Single-modality detection claimed as "confirmed"
- Detection latency exceeds modality-specific target
- RCS reduction < 10 dB
- Jamming SNR reduction < 20 dB
- Fusion confidence < 90% for known threats

## Detection Patterns

The validator detects tri-modal implementations by:
- Type names: `TriModalThreat`, `RfSignature`, `AcousticSignature`, `OpticalSignature`
- Function names: `correlate_signatures`, `fuse_modalities`, `confirm_threat`
- Variable patterns: `correlation_confidence`, `defense_layer`, `threat_type`

## Output Format

```json
{
  "scenario": "fansmitter_ac1",
  "test_duration_s": 60,
  "tests": [
    {
      "name": "acoustic_detection",
      "modality": "acoustic",
      "detected": true,
      "carrier_frequency_hz": 2400,
      "modulation": "FanRpm",
      "data_rate_bps": 0.25,
      "detection_latency_ms": 45,
      "target_latency_ms": 100,
      "status": "PASS"
    },
    {
      "name": "mmwave_detection",
      "modality": "mmwave",
      "detected": true,
      "vibration_frequency_hz": 40.0,
      "displacement_mm": 0.15,
      "detection_latency_ms": 30,
      "target_latency_ms": 50,
      "status": "PASS"
    },
    {
      "name": "cross_modal_correlation",
      "acoustic_confidence": 0.85,
      "mmwave_confidence": 0.92,
      "correlated_confidence": 0.95,
      "target_confidence": 0.8,
      "status": "PASS"
    },
    {
      "name": "threat_classification",
      "classified_as": "ConfirmedExfiltration",
      "threat_type": "Fansmitter",
      "ground_truth": "Fansmitter",
      "accuracy": true,
      "status": "PASS"
    },
    {
      "name": "jamming_effectiveness",
      "pre_jamming_snr_db": 25.0,
      "post_jamming_snr_db": 2.5,
      "snr_reduction_db": 22.5,
      "target_reduction_db": 20.0,
      "status": "PASS"
    }
  ],
  "summary": {
    "total": 5,
    "passed": 5,
    "failed": 0,
    "correlation_confidence": 0.95,
    "detection_latency_ms": 45,
    "jamming_effectiveness_db": 22.5
  }
}
```

## Tri-Modal Threat Structure

```rust
pub struct TriModalThreat {
    pub rf_signature: Option<RfSignature>,
    pub acoustic_signature: Option<AcousticSignature>,
    pub optical_signature: Option<OpticalSignature>,
    pub correlation_confidence: f32,  // 0.0 - 1.0
    pub threat_type: ThreatType,
}

pub enum ThreatType {
    KnownBenign,           // False positive
    IncidentalRadiator,    // Unintentional
    CandidateTarget,       // Needs confirmation
    ConfirmedExfiltration, // Multi-modal confirmed
    CoordinatedAttack,     // Multiple sources
}
```

## Four-Layer Defense Paradigm

```
Layer 1: SENSE
  ↓ Anomaly detected → confidence < 0.5
Layer 2: CLOAK
  ↓ RIS phase profile computed → RCS reduction > 10 dB
Layer 3: REFLECT
  ↓ False target injected → attacker confused
Layer 4: DENY
  ↓ Active jamming → SNR reduction > 20 dB
```

## Cross-Modal Correlation Algorithm

```python
def correlate_signatures(rf, acoustic, optical):
    modalities = sum([rf is not None, acoustic is not None, optical is not None])
    
    if modalities >= 2:
        # Multi-modal: high confidence
        confidence = 0.9
        
        # Check spatial consistency
        locations = [sig.source_location for sig in [rf, acoustic, optical] if sig]
        if all_distance_within_threshold(locations, max_distance_m=1.0):
            confidence = 0.95
    else:
        # Single modality: lower confidence
        confidence = 0.5
    
    return confidence
```

## Detection Latency Requirements

| Modality | Target | Critical Path |
|----------|--------|---------------|
| RF | < 200 ms | FFT → IQUMamba → Classification |
| Acoustic | < 100 ms | FFT → Feature extraction → Classification |
| Optical | < 50 ms | Frame capture → LED detection → Classification |
| mmWave | < 50 ms | UART read → Parse → Correlation |

## Timeout

Maximum execution time: 60 seconds

## Integration

This skill is called automatically by validation hooks after editing:
- `domains/spectrum/shield/src/tri_modal/**/*.rs`
- `domains/spectrum/shield/src/hal/mmwave/**/*.rs`
- Any file containing `TriModalThreat` or `correlate_signatures`

## Related Files

- `scripts/validate_tri_modal.py` - Main tri-modal validator
- `domains/spectrum/shield/src/tri_modal/` - Tri-modal implementation
- `conductor/tracks/shield_active_denial/plan.md` - Scenario definitions

## Scenario Catalog

| Scenario | Modalities | Threat Type | Target |
|----------|------------|-------------|--------|
| RF-1 | RF + Thermal | CPU EM | Cloaking + Jamming |
| AC-1 | Acoustic + mmWave | Fansmitter | Detection + Jamming |
| AC-2 | Acoustic + mmWave | PIXHELL | Detection + ANC |
| XM-5 | RF + Acoustic + Optical | Coordinated | Forensic Replay |
| XM-7 | All (adaptive) | Adaptive Adversary | Online Retraining |

## References

- "Tri-Modal Active Denial for Air-Gapped Exfiltration", Project Oz Internal
- "Cross-Modal Sensor Fusion for Threat Detection", IEEE Sensors 2024
- "RF-BSDF Wavelength-Agnostic Scattering", Project Oz Technical Note 2026-02
