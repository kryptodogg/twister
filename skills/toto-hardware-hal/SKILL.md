# Toto Hardware HAL Skill

CPAL, libiio, Pluto+ I/Q streaming, RTL-SDR capture, hardware abstraction layers,
FFI bindings, no WGSL (CPU-only capture), real-time sample acquisition.

## Domain
- RTL-SDR capture (rtlsdr static lib, FFI)
- I/Q sample handling (complex f32)
- Frequency tuning (10 kHz - 300 MHz)
- Gain control (manual/auto)
- Sample rate configuration (up to 2.4 MS/s)
- Hardware abstraction (unified audio + RTL-SDR interface)
- libiio for PlutoSDR (future TX)

## Trigger Patterns
"RTL-SDR", "I/Q", "rtlsdr", "FFI", "hardware", "capture", "tuning",
"PlutoSDR", "libiio", "HAL", "rtlsdr.rs"

## Available Functions
- `create_rtlsdr_device()` — Open and configure RTL-SDR
- `read_iq_samples()` — Capture I/Q buffer
- `set_center_freq()` — Tune to frequency
- `set_sample_rate()` — Configure sample rate
- `iq_i8_to_f32()` — Format conversion
- `link_rtlsdr_static()` — Build script linking

## Constants
- `RTLSDR_MAX_SAMPLE_RATE = 2_400_000` (2.4 MS/s)
- `RTLSDR_MIN_FREQ = 10_000` (10 kHz)
- `RTLSDR_MAX_FREQ = 300_000_000` (300 MHz)
- `RTLSDR_DEFAULT_GAIN = -1` (Auto gain)

## Code Patterns

### FFI Binding Pattern
```rust
#[repr(C)]
pub struct rtlsdr_dev_t { /* opaque */ }

extern "C" {
    pub fn rtlsdr_open(dev: *mut *mut rtlsdr_dev_t, index: u32) -> i32;
    pub fn rtlsdr_read_sync(dev: *mut rtlsdr_dev_t, buf: *mut u8, len: i32, n_read: *mut i32) -> i32;
}
```

### I/Q Conversion (i8 → f32 complex)
```rust
// i8 samples: [I0, Q0, I1, Q1, ...]
// f32 complex: [(I0/128, Q0/128), (I1/128, Q1/128), ...]
```

### Build Script Linking
```rust
// build.rs
println!("cargo:rustc-link-search=native=drivers/RTL-SDR-x64");
println!("cargo:rustc-link-lib=static=rtlsdr");
```
