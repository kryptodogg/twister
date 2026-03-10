# Skill: run_hitl_sandbox

## Overview

Allows the agent to request the human operator to boot a specific interactive target for subjective validation of visual fidelity or haptic "feel". This is a Human-In-The-Loop (HITL) skill for metrics that cannot be automatically verified.

## Applicable Agents

- `synesthesia-ui-designer`
- `siren-extreme-dsp`

## Execution

```bash
# Request HITL validation
task run-sandbox TARGET=<TARGET_NAME> OPERATOR=<OPERATOR_ID> METRIC=<METRIC_TYPE>

# Example
task run-sandbox TARGET=phased_array OPERATOR=engineer_01 METRIC=holographic_fidelity
task run-sandbox TARGET=haptic_feedback OPERATOR=engineer_02 METRIC=600hz_feel
```

## Validation Criteria

### Subjective Metrics

| Metric | Description | Scale |
|--------|-------------|-------|
| `holographic_fidelity` | Visual quality of holographic UI projection | 1-10 |
| `600hz_feel` | Smoothness of 600Hz haptic updates | 1-10 |
| `particle_smoothness` | Visual smoothness of particle animation | 1-10 |
| `audio_quality` | Perceived audio fidelity at 192kHz | 1-10 |
| `latency_perception` | Perceived system responsiveness | 1-10 |

### Pass Conditions
- Operator rating >= 7/10 for the specified metric
- No critical bugs reported during session
- Operator confirms "ready for merge"

### Fail Conditions
- Operator rating < 7/10
- Critical bugs identified
- Operator requests changes

## Interaction Flow

1. **Agent Request**: Agent initiates HITL session with target and metric
2. **Human Boot**: Operator boots the interactive sandbox target
3. **Evaluation Period**: Operator evaluates the metric (5-15 minutes)
4. **Feedback Submission**: Operator submits rating and comments
5. **Result Processing**: Agent receives feedback and decides next steps

## Output Format

```json
{
  "session_id": "hitl_20260221_143000",
  "target": "phased_array",
  "operator": "engineer_01",
  "metric": "holographic_fidelity",
  "rating": 8,
  "comments": "Holographic projection looks good. Scanline effect is visible but not distracting. Bloom intensity could be reduced by 10%.",
  "ready_for_merge": true,
  "timestamp": "2026-02-21T14:45:00Z"
}
```

## Timeout

Maximum evaluation time: 30 minutes (configurable by operator)

## Integration

This skill is manually invoked by agents when:
- Visual fidelity cannot be automatically verified
- Haptic "feel" requires human perception
- User experience validation is needed
- Final approval before merge is required

## Related Files

- `utils/watch_jules.py` - Session monitoring
- `jules_manifest.json` - Session tracking

## Example Session

```
Agent: Requesting HITL validation for holographic UI projection
       Target: phased_array
       Metric: holographic_fidelity
       Operator: engineer_01

[Operator boots sandbox target...]
[Operator evaluates for 10 minutes...]

Operator: Rating: 8/10
          Comments: Scanline effect visible but acceptable.
                    Bloom intensity could be reduced 10%.
          Ready for merge: YES

Agent: HITL validation PASSED
       Proceeding with commit...
```

## Natural Language Webhook

The operator can respond via:
- Direct terminal input
- Web interface webhook
- Chat integration (Discord, Slack)

Response format is flexible - the system parses natural language for:
- Numeric rating (1-10)
- Go/no-go decision
- Specific feedback items
