# Skill: integrate_mmwave_sensor

## Overview

Integrates XIAO MR60BHA2 60GHz mmWave sensor into SHIELD HAL for cybersecurity/defense applications. Provides UART driver, BLE bridge, point cloud visualization, and threat detection fusion (Fansmitter, PIXHELL, presence-based access).

## Applicable Agents

- `mmwave-fusion-specialist`
- `tri-modal-defense-specialist`
- `shield-rf-scientist`

## Execution

```bash
# Create mmWave HAL skeleton
node .qwen/skills/run.js integrate_mmwave_sensor --mode skeleton

# Generate Rust UART driver
node .qwen/skills/run.js integrate_mmwave_sensor --mode driver --output domains/spectrum/shield/src/hal/mmwave/

# Generate threat detection fusion
node .qwen/skills/run.js integrate_mmwave_sensor --mode fusion --scenarios fansmitter,pixhell,access_control
```

## Validation Criteria

### Pass Conditions
- UART driver correctly parses 20-byte frames with CRC16 validation
- BLE bridge forwards data bidirectionally (UART ↔ BLE NOTIFY)
- Point cloud structure compatible with Aether particle engine
- Threat detection latency < 50 ms
- False positive rate < 5% over 1-hour baseline

### Fail Conditions
- CRC validation fails on valid frames
- BLE connection drops > 10% of sessions
- Point cloud parsing errors
- Threat detection latency > 100 ms

## Detection Patterns

The skill detects mmWave integration by:
- Module names: `mmwave`, `mr60bha2`, `radar_`
- Frame patterns: `0x53` header, `0x54 0x0D` footer
- Data structures: `MmWaveFrame`, `MotionData`, `BreathHeartData`

## Output Format

```json
{
  "skill": "integrate_mmwave_sensor",
  "mode": "driver",
  "files_created": [
    "domains/spectrum/shield/src/hal/mmwave/mod.rs",
    "domains/spectrum/shield/src/hal/mmwave/mr60bha2.rs",
    "domains/spectrum/shield/src/hal/mmwave/types.rs"
  ],
  "tests": [
    {
      "name": "uart_frame_parsing",
      "input": "53 01 0E 00 ... 54 0D",
      "expected": {"breathing": 60.0, "heart": 72.0},
      "status": "PASS"
    },
    {
      "name": "crc16_validation",
      "input": "valid_frame",
      "expected": true,
      "status": "PASS"
    },
    {
      "name": "threat_detection_latency",
      "scenario": "fansmitter",
      "measured_ms": 35,
      "target_ms": 50,
      "status": "PASS"
    }
  ],
  "summary": {
    "total": 3,
    "passed": 3,
    "failed": 0,
    "detection_latency_ms": 35
  }
}
```

## mmWave Frame Format

```
Byte 0:   Header (0x53)
Byte 1:   Frame ID (0x01 = breath/heart, 0x02 = motion, 0x03 = fall)
Byte 2-3: Payload length (little-endian uint16)
Byte 4-7: Timestamp (uint32, ms since boot)
Byte 8-15: Target data (varies by frame type)
Byte 16-17: CRC16 (CCITT)
Byte 18-19: Footer (0x54, 0x0D)
```

## Rust HAL Structure

```rust
// domains/spectrum/shield/src/hal/mmwave/mod.rs
pub mod mr60bha2;
pub mod types;
pub mod fusion;

pub use mr60bha2::MmWaveSensor;
pub use types::{MmWaveFrame, MotionData, BreathHeartData};
pub use fusion::{ThreatDetector, ThreatType};
```

```rust
// domains/spectrum/shield/src/hal/mmwave/mr60bha2.rs
use tokio_serial::SerialPortBuilder;
use crate::hal::mmwave::types::*;

pub struct MmWaveSensor {
    port: tokio_serial::SerialStream,
    frame_buffer: [u8; 20],
    last_valid_frame: Option<MmWaveFrame>,
}

impl MmWaveSensor {
    pub async fn new(port_name: &str, baud: u32) -> Result<Self>;
    pub async fn read_frame(&mut self) -> Result<MmWaveFrame>;
    pub fn get_motion_data(&self) -> Option<MotionData>;
    pub fn get_vitals(&self) -> Option<BreathHeartData>;
}
```

## Threat Detection Algorithms

### Fansmitter Detection

```rust
pub fn detect_fansmitter(&self, motion: &MotionData, acoustic_rpm: f32) -> bool {
    // Convert radar velocity to estimated RPM (assuming 10cm fan blade)
    let radar_rpm = motion.velocity_cm_s as f32 * 60.0 / (2.0 * PI * 0.1);
    
    // Check if vibration energy exceeds threshold
    if motion.energy < self.config.fansmitter.energy_thresh {
        return false;
    }
    
    // Correlate with acoustic RPM detection
    let rpm_diff = (radar_rpm - acoustic_rpm).abs();
    rpm_diff < self.config.fansmitter.rpm_tolerance
}
```

### PIXHELL Detection

```rust
pub fn detect_pixhell(&self, motion: &MotionData) -> bool {
    // Check distance matches expected LCD position (±5cm)
    let distance_match = (motion.distance_mm as i32 - self.config.pixhell.lcd_distance_mm).abs() < 50;
    
    // Check for high-frequency vibration signature
    let high_freq_vibration = motion.energy > 100 && motion.velocity_cm_s > 5;
    
    distance_match && high_freq_vibration
}
```

## Point Cloud Integration

```rust
// Convert mmWave motion data to Aether particle format
pub fn motion_to_particle(&self, motion: &MotionData) -> GpuParticle {
    GpuParticle {
        position: Vec4::new(
            motion.distance_mm as f32 * motion.angle_deg as f32 / 57.29,  // X from angle
            0.0,  // Y (flat plane)
            motion.distance_mm as f32 / 1000.0,  // Z in meters
            0.0,  // W padding
        ),
        velocity: Vec4::new(
            motion.velocity_cm_s as f32 / 100.0,  // m/s
            0.0,
            0.0,
            0.0,
        ),
        state: Vec4::new(
            motion.energy as f32 / 255.0,  // Amplitude
            motion.confidence as f32 / 100.0,  // Life
            0.0,  // Frequency (unused)
            1.0,  // Status (active)
        ),
        // ... color, phasor, FLE fields
    }
}
```

## BLE-UART Bridge

```rust
use btleplug::api::{Central, Manager, Peripheral, WriteType};

pub struct BleUartBridge {
    adapter: BleAdapter,
    tx_characteristic: Characteristic,
    rx_characteristic: Characteristic,
}

impl BleUartBridge {
    pub async fn connect(device_name: &str) -> Result<Self>;
    pub async fn forward_uart_to_ble(&mut self, uart_data: &[u8]);
    pub async fn forward_ble_to_uart(&mut self, ble_data: &[u8]);
}

// Nordic UART Service UUIDs
const SERVICE_UUID: &str = "6E400001-B5A3-F393-E0A9-E50E24DCCA9E";
const TX_UUID: &str = "6E400003-B5A3-F393-E0A9-E50E24DCCA9E";
const RX_UUID: &str = "6E400002-B5A3-F393-E0A9-E50E24DCCA9E";
```

## Timeout

Maximum execution time: 60 seconds

## Integration

This skill is called automatically when:
- Creating files in `domains/spectrum/shield/src/hal/mmwave/`
- Mentioning "mmWave", "MR60BHA2", "radar integration"
- Activating `mmwave-fusion-specialist` agent

## Related Files

- `docs/mmwave/MR60BHA2_REFERENCE.md` - Complete hardware reference
- `domains/spectrum/shield/src/hal/mmwave/` - HAL implementation
- `domains/spectrum/shield/src/tri_modal/fusion.rs` - Multi-modal threat detection
- `firmware/mr60bha2/mr60bha2.ino` - ESP32-C6 firmware

## References

- [MR60BHA2 Reference](docs/mmwave/MR60BHA2_REFERENCE.md)
- [Seeed mmWave Library](https://github.com/Love4yzp/Seeed-mmWave-library)
- [NimBLE-Arduino](https://github.com/h2zero/NimBLE-Arduino)
- [tokio-serial](https://docs.rs/tokio-serial/latest/)
- [btleplug](https://docs.rs/btleplug/latest/)
