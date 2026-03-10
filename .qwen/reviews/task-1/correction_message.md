## Supervisor Review: Task 1/15 (toto crate) - Detailed Technical Correction Required

**Status:** NEEDS_CORRECTION

**Session:** 12557629327452439177
**PR:** #210 - Fix toto crate: Add bytemuck support and RDNA 2 aligned structs

---

## 1. Crate Specification: toto (Hardware Abstraction Layer)

**Domain:** 24-bit/192kHz audio ADC, Pluto+ I/Q polling, hardware device enumeration
**Constraint:** NO WGSL, NO wgpu - Pure Rust only
**Target Hardware:** ASUS TUF B550M-Plus WiFi II (Realtek 7.1-channel HD Audio CODEC)

---

## 2. Variable Fixes Required

### 2.1 Missing `use bytemuck::Pod` Import

**File:** `crates/toto/src/hal/audio_device.rs` (or equivalent)

**Current:**
```rust
use bytemuck::Zeroable;
// Missing: use bytemuck::Pod;
```

**Required:**
```rust
use bytemuck::{Pod, Zeroable};
```

**Reason:** All `#[repr(C)]` structs that are plain-old-data must implement both `Pod` and `Zeroable` for zero-copy GPU transfers.

---

### 2.2 Underscore Prefix for Unused Variables

**Warning Pattern:** `unused_variables: 14 warnings`

**Fix:** Prefix all unused variables with underscore:

```rust
// BEFORE (generates warning)
let config = AudioConfig::default();
let device_id = String::new();

// AFTER (no warning)
let _config = AudioConfig::default();
let _device_id = String::new();
```

---

## 3. Struct Definitions - 128-Byte Alignment

### 3.1 AudioBuffer Struct (CRITICAL)

**Requirement:** Must be exactly 128 bytes for RDNA 2 cache line alignment

**Current (INCORRECT - 96 bytes):**
```rust
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct AudioBuffer {
    pub sample_rate: u32,        // 4 bytes
    pub bit_depth: u32,          // 4 bytes
    pub channel_count: u32,      // 4 bytes
    pub buffer_size: u32,        // 4 bytes
    pub samples: [f32; 1024],    // 4096 bytes - WRONG, too large for HAL struct
    // Total: 4112 bytes (not 128-byte aligned)
}
```

**Required (CORRECT - 128 bytes):**
```rust
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct AudioBuffer {
    // Header (16 bytes)
    pub sample_rate: u32,        // 4 bytes - 192000 for high-res
    pub bit_depth: u32,          // 4 bytes - 24 for studio quality
    pub channel_count: u32,      // 4 bytes - 8 for 7.1 surround
    pub buffer_size: u32,        // 4 bytes - frames per buffer
    
    // Metadata (32 bytes)
    pub device_id: [u8; 32],     // 32 bytes - null-terminated device name
    
    // Timing (16 bytes)
    pub timestamp_ns: u64,       // 8 bytes - nanosecond precision
    pub frame_index: u64,        // 8 bytes - monotonic frame counter
    
    // Control (16 bytes)
    pub gain_db: f32,            // 4 bytes - preamp gain
    pub mute: u8,                // 4 bytes - bool replaced with u8 (FFI)
    pub _pad_mute: [u8; 3],      // 3 bytes - alignment padding
    
    // Reserved for HAL expansion (48 bytes)
    pub _reserved: [u8; 48],     // 48 bytes - future-proofing
    
    // TOTAL: 128 bytes (verified by static_assert)
}

// Compile-time size verification
const _: () = assert!(std::mem::size_of::<AudioBuffer>() == 128);
const _: () = assert!(std::mem::align_of::<AudioBuffer>() == 128);
```

---

### 3.2 SdrConfig Struct (CRITICAL)

**Requirement:** Must be exactly 128 bytes for Pluto+ SDR configuration

**Required:**
```rust
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct SdrConfig {
    // RF Front-End (16 bytes)
    pub center_freq_hz: f32,     // 4 bytes - e.g., 100_000_000 (100 MHz)
    pub sample_rate_hz: f32,     // 4 bytes - e.g., 2_400_000 (2.4 MSPS)
    pub bandwidth_hz: f32,       // 4 bytes - e.g., 20_000_000 (20 MHz)
    pub gain_db: f32,            // 4 bytes - LNA gain
    
    // Pluto+ Device (32 bytes)
    pub uri: [u8; 32],           // 32 bytes - "ip:192.168.2.1" or "usb:"
    
    // I/Q Stream (24 bytes)
    pub iq_format: u32,          // 4 bytes - 0=I16, 1=F32
    pub channel: u32,            // 4 bytes - 0=RX, 1=TX
    pub buffer_frames: u32,      // 4 bytes - frames per buffer
    pub _pad_iq: [u8; 12],       // 12 bytes - alignment
    
    // Control (16 bytes)
    pub enabled: u8,             // 1 byte - bool as u8
    pub loopback: u8,            // 1 byte - test mode
    pub _pad_ctrl: [u8; 14],     // 14 bytes - alignment
    
    // Reserved (40 bytes)
    pub _reserved: [u8; 40],     // 40 bytes - future expansion
    
    // TOTAL: 128 bytes
}

const _: () = assert!(std::mem::size_of::<SdrConfig>() == 128);
```

---

### 3.3 HardwareCaps Struct (CRITICAL)

**Requirement:** Must be exactly 256 bytes (2 cache lines) for capability enumeration

**Required:**
```rust
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct HardwareCaps {
    // Audio Capabilities (64 bytes)
    pub audio_sample_rates: [u32; 8],  // 32 bytes - supported rates
    pub audio_bit_depths: [u32; 4],    // 16 bytes - 16, 24, 32
    pub audio_max_channels: u32,       // 4 bytes - 8 for 7.1
    pub audio_has_high_res: u8,        // 1 byte - 192kHz support
    pub _pad_audio: [u8; 27],          // 27 bytes - alignment
    
    // SDR Capabilities (64 bytes)
    pub sdr_freq_min_hz: f32,          // 4 bytes - e.g., 70e6
    pub sdr_freq_max_hz: f32,          // 4 bytes - e.g., 6e9
    pub sdr_max_sample_rate: f32,      // 4 bytes - e.g., 61.44e6
    pub sdr_has_iq_balance: u8,        // 1 byte - calibration
    pub sdr_has_dc_offset: u8,         // 1 byte - calibration
    pub _pad_sdr: [u8; 50],            // 50 bytes - alignment
    
    // Device Info (64 bytes)
    pub vendor_id: u16,                // 2 bytes - USB VID
    pub product_id: u16,               // 2 bytes - USB PID
    pub device_revision: u16,          // 2 bytes - HW revision
    pub driver_version: [u8; 16],      // 16 bytes - driver string
    pub _pad_device: [u8; 42],         // 42 bytes - alignment
    
    // Reserved (64 bytes)
    pub _reserved: [u8; 64],           // 64 bytes - future
    
    // TOTAL: 256 bytes (2 * 128-byte cache lines)
}

const _: () = assert!(std::mem::size_of::<HardwareCaps>() == 256);
```

---

## 4. bool to u8 Replacement

**Rule:** All `bool` fields in `#[repr(C)]` structs must be `u8` for FFI compatibility.

**BEFORE (INCORRECT):**
```rust
#[repr(C)]
pub struct AudioConfig {
    pub enabled: bool,        // WRONG - bool has undefined size in FFI
    pub mute: bool,           // WRONG
}
```

**AFTER (CORRECT):**
```rust
#[repr(C)]
pub struct AudioConfig {
    pub enabled: u8,          // CORRECT - 1 byte, explicit
    pub mute: u8,             // CORRECT - 1 byte, explicit
    pub _pad: [u8; 2],        // Padding for 4-byte alignment
}
```

---

## 5. Verification Commands

After applying corrections, run:

```bash
# Verify compilation
cargo check -p toto

# Verify alignment
cargo run --manifest-path scripts/Cargo.toml --bin static_align_check -- --input crates/toto/src/ --output crates/toto/alignment.json

# Verify no bool in GPU structs
rg "bool" crates/toto/src/hal/
```

---

## 6. Reference Documents

- `docs/rdna2_infinity_cache_optimization.txt` - 128-byte alignment mandate
- `docs/cpal_high_res_audio_guide.md` - CPAL configuration
- `docs/libiio_async_rust_bindings.md` - Pluto+ I/Q polling
- `.qwen/agents/toto-hardware-hal.yml` - Agent rules (NO WGSL)

---

**ACTION REQUIRED:** Correct the above issues and re-submit Task 1/15.

**Specific Deliverables:**
1. Add `use bytemuck::Pod;` import
2. Fix 14 unused variable warnings with underscore prefix
3. Implement `AudioBuffer` struct at exactly 128 bytes
4. Implement `SdrConfig` struct at exactly 128 bytes
5. Implement `HardwareCaps` struct at exactly 256 bytes
6. Replace all `bool` with `u8` in `#[repr(C)]` structs
7. Add `const _: () = assert!(size_of::<T>() == N);` for each struct

Standing by for corrected implementation.
