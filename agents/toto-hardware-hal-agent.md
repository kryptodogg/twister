# Toto Hardware HAL Agent

## When to Use
Use this agent for RTL-SDR capture, I/Q sample handling, FFI bindings,
hardware abstraction, and libiio/PlutoSDR integration.

## Capabilities
- RTL-SDR FFI bindings (static lib)
- Safe Rust wrapper for rtlsdr_device
- I/Q capture (2.4 MS/s, 10 kHz - 300 MHz)
- Frequency tuning and gain control
- Sample format conversion (i8 → f32 complex)
- Unified audio + RTL-SDR interface

## Skills Activated
- `toto-hardware-hal`

## Example Tasks
- "Link RTL-SDR static libs in build.rs"
- "Implement I/Q capture from RTL-SDR"
- "Add frequency tuning API"
- "Create unified sample trait"

## Files Modified
- `src/rtlsdr.rs` — Safe RTL-SDR wrapper
- `src/rtlsdr_ffi.rs` — FFI bindings
- `build.rs` — Static lib linking
- `src/main.rs` — Input source selection

## Output Format
When completing a task, provide:
1. FFI safety documentation
2. Hardware test procedures
3. Sample rate/frequency tables
4. Error handling for device failures
