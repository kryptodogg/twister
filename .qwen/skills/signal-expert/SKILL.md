---
name: signal-expert
description: Audio signal processing expert specializing in sub-nyquist and super-nyquist recreation, PDM haptic transmutation, and Mamba-based blind source separation (IQUMamba-1D, SSAMBA).
---

# Audio Signal Processing & Reconstruction Expert

This skill provides expert guidance for implementing advanced DSP techniques, focusing on high-fidelity signal reconstruction, extreme concurrency, and bio-aware acoustic interfacing in Project Synesthesia.

## 🎯 Core Competencies

### 1. Psychoacoustics & Bio-Aware DSP
- **Frey Effect Avoidance:** Safely mimic the microwave auditory effect using exogenous haptic actuators and localized magnetic fields, avoiding direct microwave irradiation.
- **Biometric Processing:** Capture and decode Frequency Following Response (FFR) via EEG for passive biometric authentication.
- **Magnetic Thresholds:** Keep audio-frequency magnetic fields (20Hz-20kHz) strictly below 0.1 µT near the temporal-parietal area to prevent cognitive fatigue and working memory degradation.
- **Stochastic Resonance:** Inject tuned Gaussian or dichotomous noise to lower sensory thresholds for specific receptors:
    - **Meissner Corpuscles:** 10 Hz - 50 Hz (deep rumble).
    - **Paczian Corpuscles:** 100 Hz - 300 Hz (fine vibration).

### 2. Sub-Nyquist Haptic Transmutation
- **Pulse-Density Modulation (PDM):** Abandon PCM to bypass aggressive AC-coupling and hardware high-pass filters. Represent amplitude via the temporal density of a 1-bit stream.
- **Sigma-Delta Modulators:** Run encoders at extreme oversampled carrier frequencies (192 kHz, 2.4 MHz, or 3.072 MHz). The physical mass of the Voice Coil Actuator acts as a mechanical low-pass filter, integrating the 1-bit stream into DC/0Hz displacement.

### 3. Advanced Detection & Separation
- **Phase Exploitation:** Extract micrometer structural vibrations or speech from phase angles across consecutive radar chirps using phase unwrapping, Wiener, and median filters.
- **Deep Learning Sensing:** Use Eigenvalues Local Binary Patterns Residual Networks (EL-ResNet) to detect Direct Sequence Spread Spectrum (DSSS) signals at -20 dB SNR.
- **Super-Nyquist Synthesis:** Reconstruct 28 GHz+ mmWave channels from sub-6 GHz data using Heterogeneous Graph Neural Networks (HGNN) with dynamic attention mechanisms.
- **Mamba State Space Models:** Real-time Blind Source Separation (SCBSS) using linear-time, constant-memory models with selective scan algorithms.
    - **IQUMamba-1D:** U-Net backbone for complex-valued RF streams to mitigate quadrature imbalance (2.02M params, 2.89ms latency, 32k context).
    - **SSAMBA:** Self-supervised, bidirectional audio representation.

### 4. Bare-Metal Edge Orchestration (Rust)
- **Burn & CubeCL:** Use `Burn` with `CubeCL` to write custom, highly optimized GPU compute kernels directly in Rust syntax for highly parallelized execution (e.g., Mamba selective scans).
- **Candle:** Use Hugging Face's `Candle` for minimalist, low-overhead inference tasks on CPU.
- **SPSC Ring Buffers:** Ensure the unpredictable AI runtime never blocks the high-priority `cpal` audio thread. Use strict lock-free Single-Producer, Single-Consumer atomic buffers transferring 128-byte `HeterodynePayload` structs.
- **Autonomic Thermal Guard:** Embed real-time thermal resistance/capacitance calculations to autonomously throttle PDM density and prevent Voice Coil Actuator burnout.

## 📚 Best Practices & Guidelines
1. **128-Byte Law:** Meticulously align payloads to 128-byte boundaries to match RDNA2 GPU cache lines.
2. **Bus Saturation Strategy:** Target the 192-bit system bus by using **4-bit quantization** to maximize throughput, leveraging ML to recover precision.
3. **Gain Staging:** Always normalize to full-scale (1.0) to hit the 108dB SNR floor of target hardware.
