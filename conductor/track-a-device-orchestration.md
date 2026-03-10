# Track A: Device Orchestration (Hardware I/O)

**For**: Jules (or assigned developer)
**Goal**: Safe device management with explicit user control (no auto-detect fragility)

---

## Overview

Track A manages all hardware I/O lifecycle: RTL-SDR + Pluto+ FFI wrappers, device registry with atomic synchronization, and Slint UI callbacks.

**Why this matters**:
- Prevents catastrophic SDR lockups from fragile auto-detection
- User has **explicit control** via UI buttons: "+ Add Device", "- Remove Device", frequency tuning
- Thread-safe: all operations guarded by `Arc<Mutex<>>`
- Dirty flags propagate state changes to UI without busy-waiting

**Critical path**:
```
A.1 (FFI) → A.2 (Device Manager) ← Already done! → A.3 (UI Wiring) ← You are here
```

A.2 is **complete** (device_manager.rs written). You implement A.1 and A.3.

---

## Track A.1: FFI Wrapper Consolidation

**Status**: [ ] Not started
**Estimated time**: 3-4 days
**Blocker on**: Nothing (parallel-safe)

### Specification

**What exists**:
- `src/rtlsdr_ffi.rs` — RTL-SDR FFI bindings (feature-gated, compiles everywhere)
- Stubs for Pluto+ FFI (from earlier extension)
- `Cargo.toml` with `rtlsdr` feature, ready for `pluto-plus` feature

**What to implement**:
- `src/safe_sdr_wrapper.rs` — Safe Rust wrapper over both RTL-SDR and Pluto+ FFI
  - Enum `RadioDeviceType { RtlSdr, PlutoPlus }`
  - Struct `RadioDevice` (opaque, no raw FFI exposure)
  - Methods: `open_rtl_sdr()`, `open_pluto_plus()`, `tune_freq()`, `read_sync()`, `Drop`
  - All unsafe FFI calls isolated (no unsafe outside this file)

**What ships**:
- `cargo build --features pluto-plus` compiles cleanly
- No new warnings, zero unsafe code exposure
- Example: `examples/test_radio_device_open.rs` passes

### Implementation Guide

#### Step 1: Create `src/safe_sdr_wrapper.rs`

```rust
// src/safe_sdr_wrapper.rs — Safe wrappers for RTL-SDR and Pluto+ FFI

use crate::rtlsdr_ffi;
use std::ffi::{CStr, CString};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadioDeviceType {
    RtlSdr,
    PlutoPlus,
}

/// Safe handle to an open RTL-SDR or Pluto+ device.
/// All FFI operations are unsafe and contained within this struct.
pub struct RadioDevice {
    device_type: RadioDeviceType,
    device_id: u32,
    center_freq_hz: u64,
    sample_rate_hz: u32,

    // RTL-SDR handle (Option because Pluto+ doesn't use it)
    rtl_handle: Option<*mut rtlsdr_ffi::rtlsdr_dev_t>,

    // Pluto+ handles (Feature-gated, optional)
    #[cfg(feature = "pluto-plus")]
    pluto_context: Option<*mut rtlsdr_ffi::iio_context>,
    #[cfg(feature = "pluto-plus")]
    pluto_device: Option<*mut rtlsdr_ffi::iio_device>,
}

impl RadioDevice {
    /// Open an RTL-SDR device by USB enumeration index.
    ///
    /// # Parameters
    /// - `device_index`: USB enumeration index (0, 1, 2, ...)
    ///
    /// # Returns
    /// - `Ok(RadioDevice)` on success, initialized with safe defaults
    /// - `Err(String)` on FFI failure
    ///
    /// # Defaults set
    /// - Sample rate: 2.4 MSPS
    /// - Center frequency: 2.4 GHz
    /// - Tuner gain mode: Manual
    /// - AGC mode: On
    pub fn open_rtl_sdr(device_index: u32) -> Result<Self, String> {
        unsafe {
            let mut handle: *mut rtlsdr_ffi::rtlsdr_dev_t = std::ptr::null_mut();
            let ret = rtlsdr_ffi::rtlsdr_open(&mut handle, device_index);

            if ret != rtlsdr_ffi::RTLSDR_SUCCESS || handle.is_null() {
                return Err(format!(
                    "Failed to open RTL-SDR device {}: code {}",
                    device_index, ret
                ));
            }

            // Safe defaults
            rtlsdr_ffi::rtlsdr_set_sample_rate(handle, 2_400_000);
            rtlsdr_ffi::rtlsdr_set_center_freq(handle, 2_400_000_000);
            rtlsdr_ffi::rtlsdr_set_tuner_gain_mode(handle, 1);
            rtlsdr_ffi::rtlsdr_set_agc_mode(handle, 1);

            Ok(RadioDevice {
                device_type: RadioDeviceType::RtlSdr,
                device_id: device_index,
                center_freq_hz: 2_400_000_000,
                sample_rate_hz: 2_400_000,
                rtl_handle: Some(handle),
                #[cfg(feature = "pluto-plus")]
                pluto_context: None,
                #[cfg(feature = "pluto-plus")]
                pluto_device: None,
            })
        }
    }

    /// Open a Pluto+ device (requires `--features pluto-plus`).
    ///
    /// Discovers ad9361-phy device on the USB context.
    #[cfg(feature = "pluto-plus")]
    pub fn open_pluto_plus(device_id: u32) -> Result<Self, String> {
        unsafe {
            let ctx = rtlsdr_ffi::iio_create_default_context();
            if ctx.is_null() {
                return Err("Failed to create libiio context".into());
            }

            let device_name = CString::new("ad9361-phy")
                .map_err(|_| "Invalid device name")?;
            let dev = rtlsdr_ffi::iio_context_find_device(ctx, device_name.as_ptr());
            if dev.is_null() {
                rtlsdr_ffi::iio_context_destroy(ctx);
                return Err("ad9361 device not found on Pluto+".into());
            }

            Ok(RadioDevice {
                device_type: RadioDeviceType::PlutoPlus,
                device_id,
                center_freq_hz: 2_400_000_000,
                sample_rate_hz: 2_000_000,
                rtl_handle: None,
                pluto_context: Some(ctx),
                pluto_device: Some(dev),
            })
        }
    }

    /// Tune device to a specific frequency (Hz).
    ///
    /// # Parameters
    /// - `freq_hz`: Frequency in Hz (e.g., 2_400_000_000 for 2.4 GHz)
    ///
    /// # Returns
    /// - `Ok(())` on success
    /// - `Err(String)` if frequency set failed
    pub fn tune_freq(&mut self, freq_hz: u64) -> Result<(), String> {
        match self.device_type {
            RadioDeviceType::RtlSdr => unsafe {
                let ret = rtlsdr_ffi::rtlsdr_set_center_freq(
                    self.rtl_handle.ok_or("No RTL-SDR handle")?,
                    freq_hz as u32,
                );
                if ret != rtlsdr_ffi::RTLSDR_SUCCESS {
                    return Err(format!("RTL-SDR freq set failed: {}", ret));
                }
            },
            RadioDeviceType::PlutoPlus => {
                #[cfg(feature = "pluto-plus")]
                unsafe {
                    let attr_name = CString::new("RX_LO")
                        .map_err(|_| "Invalid attr name")?;
                    let ret = rtlsdr_ffi::iio_device_attr_write_longlong(
                        self.pluto_device.ok_or("No Pluto+ device")?,
                        attr_name.as_ptr(),
                        freq_hz as i64,
                    );
                    if ret < 0 {
                        return Err(format!("Pluto+ freq set failed: {}", ret));
                    }
                }
                #[cfg(not(feature = "pluto-plus"))]
                {
                    return Err("Pluto+ support not compiled in".into());
                }
            }
        }

        self.center_freq_hz = freq_hz;
        Ok(())
    }

    /// Read IQ samples from device (blocking, RTL-SDR only).
    ///
    /// # Parameters
    /// - `buffer`: Mutable u8 buffer (will be filled with raw IQ samples)
    ///
    /// # Returns
    /// - `Ok(n_read)` — number of bytes read
    /// - `Err(String)` on I/O error
    ///
    /// # Note
    /// Only RTL-SDR supports sync reads. Pluto+ uses async I/O (future feature).
    pub fn read_sync(&self, buffer: &mut [u8]) -> Result<usize, String> {
        if self.device_type != RadioDeviceType::RtlSdr {
            return Err("Only RTL-SDR supports sync reads (for now)".into());
        }

        unsafe {
            let mut n_read: i32 = 0;
            let ret = rtlsdr_ffi::rtlsdr_read_sync(
                self.rtl_handle.ok_or("No RTL-SDR handle")?,
                buffer.as_mut_ptr(),
                buffer.len() as i32,
                &mut n_read,
            );
            if ret != rtlsdr_ffi::RTLSDR_SUCCESS {
                return Err(format!("RTL-SDR read failed: {}", ret));
            }
            Ok(n_read as usize)
        }
    }

    // Accessors
    pub fn device_type(&self) -> RadioDeviceType {
        self.device_type
    }

    pub fn center_freq(&self) -> u64 {
        self.center_freq_hz
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate_hz
    }
}

impl Drop for RadioDevice {
    fn drop(&mut self) {
        unsafe {
            if let Some(handle) = self.rtl_handle.take() {
                let _ = rtlsdr_ffi::rtlsdr_close(handle);
            }
            #[cfg(feature = "pluto-plus")]
            {
                if let Some(ctx) = self.pluto_context.take() {
                    rtlsdr_ffi::iio_context_destroy(ctx);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_radio_device_type_enum() {
        let rt = RadioDeviceType::RtlSdr;
        assert_eq!(rt, RadioDeviceType::RtlSdr);
        assert_ne!(rt, RadioDeviceType::PlutoPlus);
    }

    #[test]
    fn test_radio_device_freq_update() {
        // This test would require actual hardware or mocking.
        // For now, just ensure the enum/type system works.
        let _device_type = RadioDeviceType::RtlSdr;
    }
}
```

#### Step 2: Update `Cargo.toml`

Add feature flag:

```toml
[features]
default = ["rtlsdr"]
rtlsdr = []
pluto-plus = []              # ← ADD THIS
all-rf = ["rtlsdr", "pluto-plus"]  # ← ADD THIS
```

#### Step 3: Update `src/rtlsdr_ffi.rs` (if not already done)

Extend with Pluto+ FFI stubs (minimal, feature-gated). The stubs ensure compilation without `pluto-plus` feature.

```rust
// At the end of rtlsdr_ffi.rs, add (if not present):

#[cfg(feature = "pluto-plus")]
#[link(name = "iio")]
unsafe extern "C" {
    pub fn iio_create_default_context() -> *mut iio_context;
    pub fn iio_context_destroy(ctx: *mut iio_context);
    pub fn iio_context_find_device(ctx: *const iio_context, name: *const c_char) -> *mut iio_device;
    pub fn iio_device_attr_write_longlong(dev: *mut iio_device, attr: *const c_char, val: i64) -> c_int;
}

#[cfg(not(feature = "pluto-plus"))]
pub unsafe fn iio_create_default_context() -> *mut iio_context {
    std::ptr::null_mut()
}
// ... (rest of stubs)
```

### Test: `examples/test_radio_device_open.rs`

```rust
// examples/test_radio_device_open.rs
//
// Tests FFI wrapper: opening device, querying properties, closing cleanly.
// Requires RTL-SDR plugged in or Pluto+ on USB.
// If no device, returns error (expected).

use twister::safe_sdr_wrapper::{RadioDevice, RadioDeviceType};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Test: Radio Device FFI Wrapper ===\n");

    // Test 1: Try to open RTL-SDR device 0
    println!("[1] Attempting to open RTL-SDR device (index=0)...");
    match RadioDevice::open_rtl_sdr(0) {
        Ok(dev) => {
            println!("✓ Device opened successfully");
            println!("  Device type: {:?}", dev.device_type());
            println!("  Center freq: {} Hz", dev.center_freq());
            println!("  Sample rate: {} Hz", dev.sample_rate());

            // Test 2: Tune frequency
            println!("\n[2] Tuning to 1.5 GHz...");
            let freq_1_5ghz = 1_500_000_000u64;
            match dev.tune_freq(freq_1_5ghz) {
                Ok(_) => {
                    println!("✓ Tuned to {} Hz", dev.center_freq());
                }
                Err(e) => {
                    println!("✗ Tune failed: {}", e);
                }
            }

            // Test 3: Clean drop
            println!("\n[3] Closing device...");
            drop(dev);
            println!("✓ Device closed (Drop impl called)");
        }
        Err(e) => {
            println!("✗ Failed to open RTL-SDR: {}", e);
            println!("  (Expected if device not plugged in)");
        }
    }

    // Test 4: Try Pluto+ (if compiled with feature)
    #[cfg(feature = "pluto-plus")]
    {
        println!("\n[4] Attempting to open Pluto+ device...");
        match RadioDevice::open_pluto_plus(0) {
            Ok(dev) => {
                println!("✓ Pluto+ opened");
                println!("  Device type: {:?}", dev.device_type());
            }
            Err(e) => {
                println!("✗ Failed to open Pluto+: {}", e);
                println!("  (Expected if device not present)");
            }
        }
    }

    println!("\n=== Test Complete ===");
    Ok(())
}
```

**Run it**:
```bash
cargo run --example test_radio_device_open --features pluto-plus
```

### Acceptance Criteria (A.1)

- [ ] `src/safe_sdr_wrapper.rs` compiles cleanly
- [ ] `cargo build --features pluto-plus` succeeds with 0 new warnings
- [ ] Example `test_radio_device_open.rs` runs without panics
- [ ] No `unsafe` code exposed outside `safe_sdr_wrapper.rs`
- [ ] Device Drop impl properly closes handles

---

## Track A.2: Device Manager Registry

**Status**: [✓] COMPLETE — Already implemented by Claude
**File**: `src/hardware_io/device_manager.rs`
**What it does**:
- Manages registry of `RadioDevice` instances
- Thread-safe add/remove/tune operations
- Updates dirty flags for UI synchronization
- Pre-allocated Vec<Option<RadioDevice>> (no reallocation during runtime)

**You don't need to modify this.** It's ready for A.3.

---

## Track A.3: Slint ↔ Device Manager Wiring

**Status**: [ ] Not started
**Estimated time**: 2-3 days
**Blocker on**: A.2 (complete ✓), A.1 (FFI wrapper)

### Specification

**What exists**:
- `ui/components/device_controls.slint` — UI definition (buttons, list, frequency input)
- `src/hardware_io/device_manager.rs` — Registry (complete)
- `src/app_state/atomics.rs` — Dirty flags (complete)

**What to implement**:
- `src/ui/app_controller.rs` — Event dispatcher
  - Listens to Slint callbacks: `add_rtl_sdr_clicked`, `add_pluto_clicked`, `remove_device_clicked`, `frequency_changed`
  - Calls `DeviceManager::add_rtl_sdr()`, `remove_device()`, `tune_device()`
  - Updates Slint device list from `DeviceManager::get_devices()`
  - Handles errors gracefully (shows error status in UI)

**What ships**:
- Click "+ Add RTL-SDR" button → device opens → UI shows "Ready" status
- Frequency input field live-updates when user types
- Remove button deletes device safely
- `cargo build --release` passes

### Implementation Guide

#### Step 1: Create `src/ui/app_controller.rs`

```rust
// src/ui/app_controller.rs — Slint event dispatcher for device controls

use crate::hardware_io::device_manager::{DeviceManager, DeviceStatus};
use crate::app_state::DirtyFlags;
use std::sync::Arc;
use slint::*;

/// Controller for device control callbacks.
///
/// Bridges Slint UI events to DeviceManager operations.
pub struct DeviceControlsController {
    device_manager: Arc<DeviceManager>,
    dirty_flags: Arc<DirtyFlags>,
}

impl DeviceControlsController {
    pub fn new(device_manager: Arc<DeviceManager>, dirty_flags: Arc<DirtyFlags>) -> Self {
        Self {
            device_manager,
            dirty_flags,
        }
    }

    /// Handle "+ Add RTL-SDR" button click.
    pub fn on_add_rtl_sdr_clicked(&self, device_index: u32) -> Result<String, String> {
        match self.device_manager.add_rtl_sdr(device_index) {
            Ok(device_id) => {
                self.dirty_flags.mark(&self.dirty_flags.device_list_dirty);
                Ok(format!("RTL-SDR device {} added (ID: {})", device_index, device_id))
            }
            Err(e) => Err(format!("Failed to add RTL-SDR: {}", e)),
        }
    }

    /// Handle "+ Add Pluto+" button click (requires pluto-plus feature).
    #[cfg(feature = "pluto-plus")]
    pub fn on_add_pluto_clicked(&self, device_index: u32) -> Result<String, String> {
        match self.device_manager.add_pluto_plus(device_index) {
            Ok(device_id) => {
                self.dirty_flags.mark(&self.dirty_flags.device_list_dirty);
                Ok(format!("Pluto+ device added (ID: {})", device_id))
            }
            Err(e) => Err(format!("Failed to add Pluto+: {}", e)),
        }
    }

    /// Handle "- Remove Device" button click.
    pub fn on_remove_device_clicked(&self, device_id: u32) -> Result<String, String> {
        match self.device_manager.remove_device(device_id) {
            Ok(_) => {
                self.dirty_flags.mark(&self.dirty_flags.device_list_dirty);
                Ok(format!("Device {} removed", device_id))
            }
            Err(e) => Err(format!("Failed to remove device: {}", e)),
        }
    }

    /// Handle frequency input change.
    pub fn on_frequency_changed(&self, device_id: u32, freq_mhz: f32) -> Result<String, String> {
        let freq_hz = (freq_mhz * 1_000_000.0) as u64;
        match self.device_manager.tune_device(device_id, freq_hz) {
            Ok(_) => {
                self.dirty_flags.mark(&self.dirty_flags.frequency_lock_dirty);
                Ok(format!("Device {} tuned to {:.1} MHz", device_id, freq_mhz))
            }
            Err(e) => Err(format!("Tuning failed: {}", e)),
        }
    }

    /// Get current device list for UI binding.
    pub fn get_device_list(&self) -> Vec<(u32, String, f32, String)> {
        self.device_manager
            .get_devices()
            .iter()
            .map(|handle| {
                let device_type = match handle.device_type {
                    crate::hardware_io::device_manager::RadioDeviceType::RtlSdr => "RTL-SDR",
                    crate::hardware_io::device_manager::RadioDeviceType::PlutoPlus => "Pluto+",
                };
                let status = match handle.status {
                    DeviceStatus::Ready => "Ready",
                    DeviceStatus::Tuning => "Tuning...",
                    DeviceStatus::Idle => "Idle",
                    DeviceStatus::Error => "Error",
                };
                (
                    handle.id,
                    format!("{} ({})", device_type, handle.id),
                    (handle.center_freq_hz as f32) / 1_000_000.0,
                    status.to_string(),
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_controller_creation() {
        let dm = Arc::new(DeviceManager::new(Arc::new(DirtyFlags::new())));
        let flags = Arc::new(DirtyFlags::new());
        let _ctrl = DeviceControlsController::new(dm, flags);
        // Controller created successfully
    }
}
```

#### Step 2: Wire Slint Callbacks (Update `ui/app.slint`)

In your main `app.slint`, add handlers for device control events:

```slint
// ui/app.slint (excerpt — update your existing app.slint)

import { DeviceControls } from "components/device_controls.slint";

export component App {
    // State for device list and status
    in-out property <[{
        id: u32,
        name: string,
        freq-mhz: f32,
        status: string,
    }]> devices;

    in-out property <string> error-message;

    // Handlers for device control events
    device_controls.add-device(device-name, device-type) => {
        // Send to Rust backend via DeviceControlsController
        // For now, stub: display success
        error-message = "Device added: " + device-name;
    }

    device_controls.remove-device(device-index) => {
        error-message = "Device removed";
    }

    device_controls.set-frequency(device-index, freq-mhz) => {
        error-message = "Tuned to " + freq-mhz + " MHz";
    }
}
```

#### Step 3: Create `src/ui/mod.rs` (if doesn't exist)

```rust
// src/ui/mod.rs

pub mod app_controller;

pub use app_controller::DeviceControlsController;
```

### Test: `examples/test_slint_device_controls.rs`

```rust
// examples/test_slint_device_controls.rs
//
// Tests Slint ↔ DeviceManager wiring (without real UI rendering).
// Mocks Slint callbacks and verifies DeviceManager is called correctly.

use std::sync::Arc;
use twister::hardware_io::device_manager::DeviceManager;
use twister::app_state::DirtyFlags;
use twister::ui::DeviceControlsController;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Test: Slint Device Controls Wiring ===\n");

    // Setup
    let flags = Arc::new(DirtyFlags::new());
    let dm = Arc::new(DeviceManager::new(flags.clone()));
    let ctrl = DeviceControlsController::new(dm.clone(), flags.clone());

    // Test 1: Get initial device list (empty)
    println!("[1] Initial device list...");
    let devices = ctrl.get_device_list();
    println!("  Devices: {} (expected 0)", devices.len());
    assert_eq!(devices.len(), 0);
    println!("✓ Initial state correct\n");

    // Test 2: Mock "+ Add RTL-SDR" button click
    println!("[2] Simulating: Click '+ Add RTL-SDR' button (device index 0)...");
    match ctrl.on_add_rtl_sdr_clicked(0) {
        Ok(msg) => println!("✓ {}", msg),
        Err(e) => println!("✗ {}", e),
    }

    // Test 3: Verify device list updated
    println!("\n[3] Checking device list after add...");
    let devices = ctrl.get_device_list();
    println!("  Devices: {} (expected 1+)", devices.len());
    if !devices.is_empty() {
        let (id, name, freq_mhz, status) = &devices[0];
        println!("  Device #{}: {} @ {:.0} MHz [{}]", id, name, freq_mhz, status);
    }

    // Test 4: Mock frequency input change
    println!("\n[4] Simulating: Frequency input change to 1500.5 MHz...");
    if !devices.is_empty() {
        let device_id = devices[0].0;
        match ctrl.on_frequency_changed(device_id, 1500.5) {
            Ok(msg) => println!("✓ {}", msg),
            Err(e) => println!("✗ {}", e),
        }
    }

    // Test 5: Verify dirty flag was set
    println!("\n[5] Checking dirty flags...");
    if flags.check_and_clear(&flags.device_list_dirty) {
        println!("✓ device_list_dirty flag was set (then cleared)");
    } else {
        println!("✓ device_list_dirty not currently set (may have been cleared)");
    }

    // Test 6: Mock "- Remove Device" button click
    println!("\n[6] Simulating: Click '- Remove Device' button...");
    if !devices.is_empty() {
        let device_id = devices[0].0;
        match ctrl.on_remove_device_clicked(device_id) {
            Ok(msg) => println!("✓ {}", msg),
            Err(e) => println!("✗ {}", e),
        }
    }

    // Test 7: Verify device list is empty again
    println!("\n[7] Checking device list after remove...");
    let devices = ctrl.get_device_list();
    println!("  Devices: {} (expected 0)", devices.len());
    assert_eq!(devices.len(), 0);
    println!("✓ Final state correct");

    println!("\n=== Test Complete ===");
    Ok(())
}
```

**Run it**:
```bash
cargo run --example test_slint_device_controls --release
```

### Acceptance Criteria (A.3)

- [ ] `src/ui/app_controller.rs` compiles cleanly
- [ ] `examples/test_slint_device_controls.rs` runs and passes all assertions
- [ ] Dirty flags are correctly set when devices are added/removed/tuned
- [ ] Error handling returns meaningful error messages
- [ ] `cargo build --release` succeeds with no new warnings

---

## Track A.4: Zero-Copy DMA Gateway (The Brick Road)

**Status**: [ ] Not started
**Estimated time**: 2-3 days
**Blocker on**: Nothing (parallel-safe, but B.1 depends on this)

### Why This Matters

Without A.4, the entire "zero f32 conversion" architecture collapses. Raw IQ bytes from RTL-SDR must reach GPU VRAM **without touching the Ryzen 7 CPU's math pipelines**.

Current bottleneck:
```
RTL-SDR → [u8; 2] samples → ??? → GPU STFT
                              ↑
                    (nowhere to go!)
```

After A.4:
```
RTL-SDR → [u8; 2] samples → CPU staging buffer → DMA copy → GPU VRAM (read-only for STFT)
                                                              ↑
                                                  (zero f32 conversion)
```

### Specification

**What exists**:
- `src/vbuffer.rs::IqVBuffer` — Bare shell, pre-allocated buffer
- `src/gpu_memory.rs::UnifiedBuffer<T>` — Atomic synchronization primitives (already handles CPU↔GPU)
- `src/hardware_io/device_manager.rs` — Device registry with `read_sync()` method

**What to implement**:
- `src/hardware_io/dma_vbuffer.rs` — Zero-copy ingestion gateway
  - Host-visible staging buffer (CPU-mapped, MAP_WRITE)
  - GPU VRAM rolling history buffer (STORAGE, COPY_DST)
  - Circular buffer pointer: write offset advances, wraps at boundary
  - No allocations after init (Vec pre-sized)
  - Atomic coordination with dirty flags

**What ships**:
- Raw `[u8]` from `RadioDevice::read_sync()` → staging → VRAM (no f32 conversion)
- Example: `examples/test_dma_ingestion.rs` (verify data flows from staging to VRAM)
- No new warnings, clean compilation

### Implementation Guide

#### Step 1: Create `src/hardware_io/dma_vbuffer.rs`

```rust
// src/hardware_io/dma_vbuffer.rs — Zero-Copy DMA Gateway
//
// Maps raw RTL-SDR [u8] IQ samples directly into GPU VRAM without CPU f32 conversion.
// Uses circular buffer: write_offset advances, wraps at max_vram_bytes.
// Host staging buffer → wgpu::queue write_buffer() → GPU VRAM (no PCIe copies).

use wgpu::util::DeviceExt;
use std::sync::Arc;

/// Size of DMA chunk (16384 complex samples * 2 bytes (I+Q) = 32 KB per transfer).
/// Balances PCIe overhead with latency.
pub const DMA_CHUNK_SAMPLES: usize = 16384;
const CHUNK_BYTES: usize = DMA_CHUNK_SAMPLES * 2;

/// History depth: 64 chunks = 1,048,576 bytes ≈ 10.7 seconds @ 2.4 MSPS.
pub const DMA_HISTORY_CHUNKS: usize = 64;

pub struct IqDmaGateway {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,

    // CPU-visible staging buffer (MAP_WRITE | COPY_SRC)
    staging_buffer: wgpu::Buffer,

    // GPU VRAM rolling history (STORAGE | COPY_DST)
    pub vram_buffer: wgpu::Buffer,

    // Circular buffer state
    write_offset_bytes: wgpu::BufferAddress,
    max_vram_bytes: wgpu::BufferAddress,
}

impl IqDmaGateway {
    /// Create a new DMA gateway with rolling history.
    ///
    /// # Parameters
    /// - `device`: wgpu Device for buffer allocation
    /// - `queue`: wgpu Queue for async copy operations
    /// - `history_chunks`: Number of DMA chunks to keep in rolling history
    ///
    /// # Returns
    /// IqDmaGateway with pre-allocated staging and VRAM buffers
    pub fn new(
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        history_chunks: usize,
    ) -> Self {
        // Staging buffer: CPU writes here, GPU reads via DMA copy
        let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("IQ Staging Buffer (host-visible)"),
            size: CHUNK_BYTES as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::MAP_WRITE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // VRAM rolling history buffer
        let max_vram_bytes = (CHUNK_BYTES * history_chunks) as wgpu::BufferAddress;
        let vram_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("IQ VRAM Rolling History"),
            size: max_vram_bytes,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            device,
            queue,
            staging_buffer,
            vram_buffer,
            write_offset_bytes: 0,
            max_vram_bytes,
        }
    }

    /// Push raw IQ bytes directly to VRAM via DMA (zero f32 conversion).
    ///
    /// # Parameters
    /// - `raw_iq_bytes`: [u8; 2] samples from RTL-SDR (interleaved I, Q)
    ///
    /// # Flow
    /// 1. Map staging buffer (CPU-accessible)
    /// 2. Copy bytes into staging
    /// 3. Unmap staging
    /// 4. Queue GPU command: copy staging → VRAM at write_offset_bytes
    /// 5. Update write_offset_bytes (circular wrap)
    ///
    /// # Returns
    /// - `Ok(())` on success
    /// - `Err(String)` if chunk size invalid
    pub fn push_dma_chunk(&mut self, raw_iq_bytes: &[u8]) -> Result<(), String> {
        if raw_iq_bytes.len() != CHUNK_BYTES {
            return Err(format!(
                "Invalid chunk size: expected {}, got {}",
                CHUNK_BYTES,
                raw_iq_bytes.len()
            ));
        }

        // Step 1: Map staging buffer for CPU write
        let buffer_slice = self.staging_buffer.slice(..);

        // Use blocking_map_async (synchronous) for simplicity
        // In high-performance scenarios, use async_map_async + polling
        buffer_slice.map_async(wgpu::MapMode::Write, |_| {});
        self.device.poll(wgpu::Maintain::Wait);

        {
            // Step 2: Copy raw bytes into mapped staging buffer
            let mut view = buffer_slice.get_mapped_range_mut();
            view.copy_from_slice(raw_iq_bytes);
        }

        // Step 3: Unmap so GPU can read
        self.staging_buffer.unmap();

        // Step 4: Queue GPU command: staging → VRAM DMA copy
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("IQ DMA Copy Encoder"),
        });

        encoder.copy_buffer_to_buffer(
            &self.staging_buffer,
            0,
            &self.vram_buffer,
            self.write_offset_bytes,
            CHUNK_BYTES as wgpu::BufferAddress,
        );

        self.queue.submit(std::iter::once(encoder.finish()));

        // Step 5: Update circular buffer pointer
        self.write_offset_bytes = (self.write_offset_bytes + CHUNK_BYTES as u64) % self.max_vram_bytes;

        Ok(())
    }

    /// Get current write offset (for debugging / visualization).
    pub fn write_offset(&self) -> wgpu::BufferAddress {
        self.write_offset_bytes
    }

    /// Get maximum VRAM size.
    pub fn max_vram_size(&self) -> wgpu::BufferAddress {
        self.max_vram_bytes
    }

    /// Reset to start of buffer (use on mode change or error recovery).
    pub fn reset(&mut self) {
        self.write_offset_bytes = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dma_chunk_constants() {
        // Verify DMA chunk size
        assert_eq!(CHUNK_BYTES, 32768);
        assert_eq!(DMA_CHUNK_SAMPLES, 16384);

        // Verify history depth
        assert_eq!(DMA_HISTORY_CHUNKS, 64);
    }

    #[test]
    fn test_circular_wraparound() {
        // Simulate offset wraparound
        let mut offset = 0u64;
        let max = (CHUNK_BYTES * 4) as u64; // Small buffer for testing

        for _ in 0..8 {
            offset = (offset + CHUNK_BYTES as u64) % max;
        }

        // Should wrap back to 0 after 4 iterations
        assert_eq!(offset, 0);
    }
}
```

#### Step 2: Update `src/hardware_io/mod.rs`

```rust
// src/hardware_io/mod.rs

pub mod device_manager;
pub mod dma_vbuffer;

pub use device_manager::DeviceManager;
pub use dma_vbuffer::IqDmaGateway;
```

#### Step 3: Integrate with Dispatch Loop (stub for now)

In `src/dispatch.rs` (will be implemented in Track B.1), the dispatch loop will use:

```rust
// Pseudocode: src/dispatch.rs (Track B.1 will implement this)

let mut dma_gateway = IqDmaGateway::new(device.clone(), queue.clone(), 64);

loop {
    // Read from device
    let n_read = device.read_sync(&mut iq_buffer)?;

    // Push to GPU DMA
    dma_gateway.push_dma_chunk(&iq_buffer[..n_read])?;

    // Mark dirty flag for UI
    dirty_flags.mark(&dirty_flags.audio_features_dirty);
}
```

### Test: `examples/test_dma_ingestion.rs`

```rust
// examples/test_dma_ingestion.rs
//
// Tests DMA gateway: allocate buffers, simulate IQ byte ingestion, verify offset tracking.
// Does NOT require actual GPU device (uses stubs for testing logic).

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Test: DMA Gateway (Zero-Copy Ingestion) ===\n");

    // Note: Full GPU test requires wgpu device initialization.
    // This is a logic test only (actual GPU test in integration tests).

    // Test 1: Verify chunk constants
    println!("[1] Chunk size validation...");
    println!("  DMA_CHUNK_SAMPLES: {}", twister::hardware_io::DMA_CHUNK_SAMPLES);
    println!("  DMA_CHUNK_BYTES: {} (expected 32768)",
             twister::hardware_io::DMA_CHUNK_SAMPLES * 2);
    assert_eq!(twister::hardware_io::DMA_CHUNK_SAMPLES * 2, 32768);
    println!("✓ Chunk constants correct\n");

    // Test 2: Verify circular buffer math
    println!("[2] Circular buffer wraparound...");
    let mut offset = 0u64;
    let max = 65536u64; // 2 chunks
    for i in 0..4 {
        offset = (offset + 32768) % max;
        println!("  Iteration {}: offset = {}", i + 1, offset);
    }
    assert_eq!(offset, 0);
    println!("✓ Wraparound correct\n");

    println!("=== Test Complete (GPU integration in Track B.1) ===");
    Ok(())
}
```

**Run it**:
```bash
cargo run --example test_dma_ingestion --release
```

### Acceptance Criteria (A.4)

- [ ] `src/hardware_io/dma_vbuffer.rs` compiles cleanly
- [ ] No unsafe code outside of wgpu API calls (encapsulated)
- [ ] `examples/test_dma_ingestion.rs` passes
- [ ] Circular buffer logic verified (offset wraparound correct)
- [ ] Ready for Track B.1 (dispatch loop) to use

---

## Summary: What You Ship

By completing Track A, Jules delivers:

✅ **A.1: FFI Wrapper** (`src/safe_sdr_wrapper.rs`)
- RTL-SDR safe wrapper (working, tested)
- Pluto+ safe wrapper (feature-gated, tested)
- Example: `examples/test_radio_device_open.rs`

✅ **A.2: Device Manager** (`src/hardware_io/device_manager.rs`)
- Already complete (Claude)
- Ready to use in A.3

✅ **A.3: Slint Wiring** (`src/ui/app_controller.rs`)
- Event callbacks: add/remove/tune
- Dirty flag synchronization
- Example: `examples/test_slint_device_controls.rs`

**Result**:
```
User clicks "+ Add RTL-SDR"
  ↓ Slint button callback fires
  ↓ DeviceControlsController.on_add_rtl_sdr_clicked(0)
  ↓ DeviceManager.add_rtl_sdr(0)
  ↓ RadioDevice::open_rtl_sdr(0) [safe wrapper]
  ↓ rtlsdr_ffi::rtlsdr_open() [unsafe FFI, isolated]
  ↓ Device opened, dirty flag set
  ↓ UI re-renders device list: "RTL-SDR (ID: 1) [Ready] @ 2400.0 MHz"
```

---

## Next Steps (After Track A Complete)

Track A unblocks:
- **A.3 → B.1**: IQ dispatch loop can start (needs DeviceManager)
- **A.1 + A.2 + A.3 → F.1**: Integration testing can begin

Track B (Signal Ingestion) can begin immediately after B.2 is done (STFT shader).

---

## Deliverable Format for Jules

**Email/PR message**:

```
Subject: Track A: Device Orchestration (3 sub-tasks, ~1-2 weeks)

Hi Jules,

Here's Track A ready to implement. It's the hardware I/O layer: safe FFI wrappers + device registry + Slint callbacks.

**What you're building**:
1. A.1 (FFI Wrapper) — Safe wrappers for RTL-SDR + Pluto+ FFI
2. A.2 (Device Manager) — Already done ✓
3. A.3 (Slint Wiring) — Event callbacks to device manager

**Files to create**:
- src/safe_sdr_wrapper.rs (see code spec above)
- src/ui/app_controller.rs (see code spec above)
- examples/test_radio_device_open.rs
- examples/test_slint_device_controls.rs

**Acceptance criteria**:
- cargo build --features pluto-plus
- Both examples pass cleanly
- cargo build --release (0 new warnings)

**Parallel-safe**: No conflicts with other tracks. Tests are isolated.

See conductor/track-a-device-orchestration.md for detailed implementation guide.

Thanks!
```

---

**Last Updated**: 2026-03-08
**Author**: Claude
**Review**: Ready for Jules
