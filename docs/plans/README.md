# Twister: Active Noise Cancellation with BSS

## Project Summary

A Rust implementation of an Active Noise Cancellation (ANC) system using Blind Source Separation (BSS) with RLS (Recursive Least Squares) adaptive filtering.

## Project Structure

```
src/
├── main.rs                 # CLI entry point
├── lib.rs                  # Library exports
├── bin/
│   ├── calibrate.rs        # Hardware calibration utility
│   └── latency_test.rs     # Latency benchmark tool
├── hardware/
│   ├── mod.rs
│   ├── traits.rs           # CaptureDevice, PlaybackDevice traits
│   ├── audio.rs            # Audio I/O (stub implementation)
│   ├── rtlsdr.rs           # RTL-SDR support (stub implementation)
│   └── calibration.rs      # Hardware calibration
├── bss/
│   ├── mod.rs
│   ├── traits.rs           # AdaptiveFilter trait
│   ├── rls.rs              # RLS adaptive filter
│   └── lms.rs              # LMS adaptive filter
├── dsp/
│   ├── mod.rs
│   ├── fft.rs              # FFT processing
│   ├── filters.rs          # FIR/IIR filters
│   └── window.rs           # Window functions
└── utils/
    ├── mod.rs
    ├── error.rs            # Error types
    ├── config.rs           # Configuration
    ├── logging.rs          # Logging setup
    └── latency.rs          # Latency monitoring
```

## Features

### Core Signal Processing
- **RLS Adaptive Filter**: Fast-converging adaptive filter for noise cancellation
- **LMS Adaptive Filter**: Simpler alternative with lower computational cost
- **FFT Processing**: Real-time spectral analysis
- **Digital Filters**: FIR and IIR filter implementations

### Hardware Support (Stub)
- Audio capture/playback via CPAL (stub for compilation)
- RTL-SDR support (stub for compilation)
- Hardware calibration utilities

### Latency Monitoring
- Real-time latency tracking
- Statistical analysis (min, max, avg, percentiles)

## Usage

```bash
# Run the main application
cargo run

# Run hardware calibration
cargo run --bin calibrate

# Run latency benchmark
cargo run --bin latency_test

# List available devices
cargo run -- list-devices
```

## Configuration

Default configuration for ANC:
- Sample Rate: 48 kHz
- Buffer Size: 256 samples (~5.3ms latency)
- Target End-to-End Latency: <35ms
- RLS Filter Order: 64 taps
- Forgetting Factor: 0.995

## Dependencies

- `cpal` - Cross-platform audio I/O
- `rtlsdr` - RTL-SDR dongle support
- `ndarray` - Numerical computing
- `num-complex` - Complex numbers for signal processing
- `rustfft` / `realfft` - FFT implementations
- `tracing` - Logging and diagnostics
- `thiserror` - Error handling
- `serde` - Configuration serialization

## Implementation Notes

### RLS Algorithm
The RLS filter minimizes the weighted least squares error:
```
J(n) = Σ λ^(n-i) * |e(i)|²
```
where λ is the forgetting factor (typically 0.98-0.999).

### Latency Budget
Target: <35ms end-to-end
- Capture: 5ms
- BSS/RLS: 10ms  
- DSP: 5ms
- Output: 5ms
- Margin: 10ms

## Next Steps

1. **Hardware Integration**: Replace stub implementations with actual CPAL/RTL-SDR code
2. **Mamba Integration**: Add state-space model for advanced signal separation
3. **Pipeline Optimization**: Implement zero-copy buffer management
4. **Testing**: Add unit tests and integration tests
5. **Documentation**: Expand API documentation and examples

## License

SHIELD Project
