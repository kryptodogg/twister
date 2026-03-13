# BetterJoy Driver Integration Guide

**Version:** 7.1  
**Source:** https://github.com/Davidobot/BetterJoy  
**Location:** `S:\shield\drivers\BetterJoy_v7.1\`

---

## Overview

BetterJoy is a Windows driver that allows Nintendo Switch controllers (Joy-Cons, Pro Controller) to work as generic XInput controllers. For Project Oz, we use BetterJoy's **CemuHook UDP telemetry** feature to broadcast IMU data (accelerometer + gyroscope) via UDP on port 26760.

---

## Installation

### Step 1: Install ViGEmBus Driver

**Location:** `drivers\BetterJoy_v7.1\Drivers\ViGEmBusSetup_x64.msi`

1. Run `ViGEmBusSetup_x64.msi` (64-bit Windows)
2. Accept license agreement
3. Complete installation wizard
4. **Restart required** - Reboot after installation

### Step 2: Install HIDGuardian (Optional - for exclusive mode)

**Location:** `drivers\BetterJoy_v7.1\Drivers\HIDGuardian\`

**Install:**
```cmd
# Run as Administrator
cd drivers\BetterJoy_v7.1\Drivers\HIDGuardian
.\HIDGuardian Install (Run as Admin).bat
```

**Uninstall:**
```cmd
# Run as Administrator
.\HIDGuardian Uninstall (Run as Admin).bat
```

### Step 3: Configure BetterJoy

**Executable:** `drivers\BetterJoy_v7.1\x64\BetterJoyForCemu.exe`

**Settings:**
1. Open BetterJoyForCemu.exe
2. Click "Settings" (gear icon)
3. Enable:
   - ✅ "Start with Windows"
   - ✅ "Show in tray"
   - ✅ "Enable UDP server" (CRITICAL for Project Oz)
   - ✅ "Motion only" (we only need IMU data)
4. UDP Server Port: `26760` (default, do not change)
5. Click "Save"

---

## CemuHook UDP Protocol

### Packet Structure (100 bytes)

| Offset | Size | Field | Type | Description |
|--------|------|-------|------|-------------|
| 0-3 | 4 | Magic | u32 LE | Always `0x00000001` |
| 4-7 | 4 | Packet counter | u32 LE | Incrementing counter |
| 8-11 | 4 | Button state | u32 LE | Button bitmask |
| 12-15 | 4 | Analog stick L | u32 LE | X/Y packed |
| 16-19 | 4 | Analog stick R | u32 LE | X/Y packed |
| 20-43 | 24 | Reserved | - | Padding |
| 44-47 | 4 | Accel X | f32 LE | Acceleration X (m/s²) |
| 48-51 | 4 | Accel Y | f32 LE | Acceleration Y (m/s²) |
| 52-55 | 4 | Accel Z | f32 LE | Acceleration Z (m/s²) |
| 56-59 | 4 | Gyro Pitch | f32 LE | Rotation around X (rad/s) |
| 60-63 | 4 | Gyro Yaw | f32 LE | Rotation around Y (rad/s) |
| 64-67 | 4 | Gyro Roll | f32 LE | Rotation around Z (rad/s) |
| 68-75 | 8 | Timestamp | u64 LE | Hardware timestamp (μs) |
| 76-99 | 24 | Reserved | - | Padding |

### UDP Endpoint

- **Address:** `127.0.0.1` (localhost)
- **Port:** `26760`
- **Protocol:** UDP
- **Broadcast:** No (unicast only)

---

## Project Oz Integration

### Auto-Start with Elevated Privileges

BetterJoy requires administrator privileges to access HID devices. Project Oz implements **TPM2 + YubiKey GOD MODE** authentication before elevating:

```rust
// domains/interface/toto/src/betterjoy.rs

use std::process::Command;
use crate::security::{SecurityContext, GodModeScope};

/// Auto-start BetterJoy if not running
/// Requires GOD MODE with "betterjoy_start" scope
pub async fn ensure_betterjoy_running(ctx: &mut SecurityContext) -> Result<(), String> {
    // Check if BetterJoy is already running
    if is_betterjoy_running() {
        return Ok(());
    }

    // Require GOD MODE elevation
    if !ctx.is_allowed("betterjoy_start") {
        return Err("GOD MODE required: betterjoy_start scope".into());
    }

    // Execute with elevation
    let betterjoy_path = std::env::current_dir()?
        .join("drivers")
        .join("BetterJoy_v7.1")
        .join("x64")
        .join("BetterJoyForCemu.exe");

    Command::new("powershell")
        .args(&[
            "-Command",
            "Start-Process",
            "-FilePath",
            betterjoy_path.to_str().unwrap(),
            "-Verb",
            "RunAs"  // Elevate to Administrator
        ])
        .status()?;

    // Wait for BetterJoy to initialize (up to 5 seconds)
    for _ in 0..50 {
        tokio::time::sleep(Duration::from_millis(100)).await;
        if is_betterjoy_running() {
            return Ok(());
        }
    }

    Err("BetterJoy failed to start".into())
}

fn is_betterjoy_running() -> bool {
    // Check for BetterJoyForCemu.exe process
    sysinfo::ProcessExt::name()
        .contains("BetterJoyForCemu")
}
```

### Security Flow

```
┌─────────────────────────────────────────────────────────────┐
│  BetterJoy Auto-Start Security Flow                         │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  1. Check if BetterJoy running                              │
│     └─→ YES: Continue normally                              │
│     └─→ NO: Request elevation                               │
│                                                             │
│  2. GOD MODE Authentication Required                        │
│     └─→ SAFE MODE: DENY (log attempt)                       │
│     └─→ GOD MODE: Check scope                               │
│         └─→ No "betterjoy_start" scope: DENY               │
│         └─→ Has scope: Proceed                              │
│                                                             │
│  3. TPM2 + YubiKey Challenge-Response                       │
│     └─→ Insert YubiKey                                      │
│     └─→ Touch YubiKey (physical presence)                   │
│     └─→ TPM2 unseals encryption key                         │
│     └─→ GOD MODE granted (timed expiration)                 │
│                                                             │
│  4. Elevate BetterJoy                                       │
│     └─→ PowerShell Start-Process -Verb RunAs               │
│     └─→ UAC prompt (Windows)                                │
│     └─→ BetterJoy starts with admin rights                  │
│                                                             │
│  5. Audit Log                                               │
│     └─→ Timestamp, user, scope, YubiKey serial             │
│     └─→ Immutable log (TPM2-sealed)                         │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### Configuration

**BetterJoy config file:** `drivers\BetterJoy_v7.1\settings`

```json
{
  "udp_server": true,
  "udp_port": 26760,
  "motion_only": true,
  "start_minimized": true,
  "autostart": false  // We handle autostart via Project Oz
}
```

---

## Troubleshooting

### Issue: BetterJoy won't start

**Solution:**
1. Check ViGEmBus is installed
2. Run as Administrator manually once
3. Check Windows Event Viewer for errors

### Issue: No UDP packets received

**Solution:**
1. Verify "Enable UDP server" is checked in BetterJoy settings
2. Check firewall: Allow `BetterJoyForCemu.exe` on port 26760
3. Test with netcat: `nc -u -l 26760`

### Issue: Joy-Cons disconnect frequently

**Solution:**
1. Disable Windows Bluetooth power saving (see CLI_STANDARDS.md Appendix C)
2. Use external Bluetooth 5.0+ adapter
3. Keep Joy-Cons within 3 feet of adapter

### Issue: HIDGuardian blocks controllers

**Solution:**
1. Whitelist BetterJoy in HIDGuardian:
   ```cmd
   # Run as Administrator
   cd drivers\BetterJoy_v7.1\Drivers\HIDGuardian\_drivers\HidCerberus.Srv
   # Open browser to localhost, add BetterJoy to whitelist
   ```

---

## License

BetterJoy is licensed under the MIT License. See `drivers\BetterJoy_v7.1\LICENSE`.

**Attribution:**
- **Author:** Davidobot
- **Repository:** https://github.com/Davidobot/BetterJoy
- **Version:** 7.1

---

## Related Documentation

- `docs/CLI_STANDARDS.md` - Bluetooth pairing guide
- `conductor/tracks/toto_telemetry/` - Implementation track
- `domains/interface/toto/src/telemetry.rs` - UDP listener code
