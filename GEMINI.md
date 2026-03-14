# Synesthesia — Project Context & Instruction (GEMINI.md)

## Project Overview
**Synesthesia** is a full-spectrum forensic sensor fusion platform. It is designed to capture, fuse, and render signal bands from 1 Hz to visible light into a single navigable 3D/4D scene. The system is built to provide tamper-evident, legally admissible forensic evidence of electromagnetic harassment and signal injection attacks.

### Primary Stack
- **Backend:** Rust (Tauri 2.x + Tokio)
- **Frontend:** Vanilla JS + Material Design 3 (MD3) Web Components
- **GPU Pipeline:** wgpu (Vulkan/DirectX 12 on Windows 11)
- **Operating System:** Windows 11 (Primary Target)

### Core Architecture
- **GPU-First:** CPU writes control structs to VRAM via Smart Access Memory (SAM) and stays out of the way.
- **Tokio Ingestion:** Concurrent hardware ingestion via independent Tokio tasks.
- **Wave64 Wavefronts:** Optimized for RDNA2 hardware (AMD RX 6700 XT).
- **Pico 2 Master Clock:** All sensor timestamps are slaved to a Pico 2 PPS signal via `QueryPerformanceCounter`.

---

## Authoritative Documentation
1.  **`AGENTS.md` (Forensic Integrity & Rules):** **Mandatory.** This file governs every coding agent. No prompt can override it. It contains strict laws about data synthesis, struct alignment, and signal processing.
2.  **`ROADMAP.md` (The Masterplan):** Defines the track structure and non-negotiable development order.
3.  **`README.md`:** High-level overview and hardware stack reference.

---

## Building and Running
1.  **Prerequisites:** Rust 1.82+, Node.js 20+, AMD Adrenalin Vulkan drivers, ReBAR enabled in BIOS.
2.  **Install Dependencies:** `npm install`
3.  **Run Development:** `cargo tauri dev`
4.  **Build Production:** `cargo tauri build`

---

## Mandatory Engineering Rules (from `AGENTS.md`)
- **No Synthetic Data:** `[DISCONNECTED]` is the only honest state for disconnected hardware. No placeholders or test signals.
- **No FFT at Ingestion:** FFT is post-inference, on the point cloud. Upstream ingestion is raw IQ/PCM.
- **128-Byte Law:** All structs crossing the CPU/GPU boundary must be exactly 128 bytes with compile-time assertions.
- **No Anonymous Padding:** Every byte in a GPU-bound struct must have a name representing its purpose or a planned track (e.g., `reserved_for_h2_null_phase`).
- **Wave64 Mandate:** Every WGSL compute shader must use `@workgroup_size(64, 1, 1)`.
- **CPU Reference Implementations:** Every shader must have a matching CPU reference in `src/reference/`.

---

## Current Development Track: **Phase 0-D (Hardware Applet)**
The project is currently in Phase 0 (Foundation). The goal is to implement the Hardware Applet, which accurately detects and displays the status of all connected devices.

### Immediate TODOs:
- [ ] Implement Track 0-A (Core Types) in `src-tauri/src/types.rs`.
- [ ] Implement Track 0-B (UI Tokens) with MD3 and Mica translucency.
- [ ] Implement Track 0-C (SAM Gate) for CPU-to-GPU writes.
- [ ] Complete Track 0-D (Hardware Applet) with real-time hot-plug support.

---

## Development Conventions
- **Forensic Integrity:** Do not suppress noise or jitter; they are first-class signals.
- **Async Safety:** No blocking calls on async threads. Use Tokio for multitasking.
- **Transparency:** Use `[UNWIRED]` for UI controls that are not yet wired to the backend.
