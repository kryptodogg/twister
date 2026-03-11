# SIREN v0.5 - Complete Feature Implementation

**Purpose:** Full implementation plan for all features and unused code

**Date:** 2025-03-06  
**Time Budget:** 6-8 hours (focused session)

---

## Current State Assessment

### Working (No Changes Needed)
- ✅ Audio capture (192 kHz, 24-bit, Realtek ALC S1200A)
- ✅ RTL-SDR hardware detection (Blog V4)
- ✅ GPU compute (RX 6700 XT, Vulkan)
- ✅ Bispectrum detection
- ✅ Forensic logging with UTC timestamps
- ✅ Evidence report export (HTML + CSV)
- ✅ Mamba inference (runs, produces anomaly scores)
- ✅ UI launches

### Broken (Must Fix)
- ❌ Mamba training (loss=0.0000 - not learning)
- ❌ Waterfall visualization (computed but not displayed)
- ❌ Spectrum bars (computed but not displayed)
- ❌ 112 compiler warnings

### Missing (Nice to Have)
- ⏳ Active denial/masking
- ⏳ Harmonic tracker
- ⏳ Signal triangulation
- ⏳ Continuous monitoring mode

---

## Phase 1: Critical Fixes (2-3 hours)

### 1.1 Fix Mamba Training (1 hour)

**Files:** `src/training.rs`, `src/main.rs`

**Steps:**
1. Add debug logging to see if batches are received
2. Lower anomaly threshold for pair collection (from -20 dB to 10 dB)
3. Verify gradient computation

**Code Changes:**

`src/training.rs` line ~220:
```rust
eprintln!("[Mamba] Batch received: {} pairs", batch.len());
```

`src/main.rs` line ~365:
```rust
let threshold = 10.0; // Was -20.0
```

**Success:** Console shows `Loss: 0.xxxx` (non-zero)

---

### 1.2 Wire Waterfall Display (30 min)

**Files:** `src/main.rs`

**Add to timer callback (line ~460):**
```rust
if let Ok(wf) = state.waterfall_rgba.lock() {
    let img = slint::Image::from_rgba8(
        slint::SharedPixelBuffer::clone_from_slice(
            wf.iter().map(|&rgba| slint::Rgba8Pixel {
                r: (rgba & 0xFF) as u8,
                g: ((rgba >> 8) & 0xFF) as u8,
                b: ((rgba >> 16) & 0xFF) as u8,
                a: 255u8,
            }).collect::<Vec<_>>().as_slice(),
            128, 64
        )
    );
    ui.set_waterfall_image(img);
}
```

**Success:** UI shows updating waterfall

---

### 1.3 Wire Spectrum Bars (30 min)

**Files:** `src/main.rs`

**Add after waterfall code:**
```rust
if let Ok(bars) = state.spectrum_bars.lock() {
    let mut path = String::from("M 0 100");
    for (i, &mag) in bars.iter().enumerate() {
        let x = (i as f32 / 256.0) * 1000.0;
        let y = 100.0 - (mag.clamp(0.0, 1.0) * 100.0);
        path.push_str(&format!(" L {} {}", x, y));
    }
    path.push_str(" L 1000 100 Z");
    ui.set_spectrum_path_green(path.into());
}
```

**Success:** UI shows spectrum bars

---

### 1.4 Fix Top 50 Compiler Warnings (1 hour)

**Files:** `src/main.rs`, `src/mamba.rs`, `src/anc.rs`, `src/rtlsdr.rs`

**Quick Wins:**

1. **Remove unused imports** (10 warnings):
```rust
// src/main.rs - DELETE these lines:
use crate::graph::ForensicGraph;
use crate::training::MambaTrainer;
use std::sync::Mutex;
```

2. **Prefix unused variables** (15 warnings):
```rust
// src/mamba.rs:115
let (_t, _f) = x.dims2()?;

// src/main.rs:337
let _has_events = !events.is_empty();
```

3. **Remove unnecessary `mut`** (4 warnings):
```rust
// src/main.rs:113,115,116,118 - Remove `mut` from:
let gpu_ctx = ...
let pdm = ...
let waterfall = ...
let bispec = ...
```

4. **Add `#[allow(dead_code)]`** (21 warnings for structs):
```rust
// src/anc.rs:67
#[allow(dead_code)]
pub struct AcousticTransfer { ... }

// src/anc.rs:86
#[allow(dead_code)]
pub struct PhaseCalibrator { ... }

// src/anc.rs:332
#[allow(dead_code)]
pub struct LmsFilter { ... }

// src/anc.rs:486
#[allow(dead_code)]
pub struct AncEngine { ... }
```

**Success:** <60 warnings (down from 112)

---

## Phase 2: RTL-SDR Integration (1-2 hours)

### 2.1 Wire RTL-SDR Data (1 hour)

**Files:** `src/main.rs`, `src/sdr.rs`

**Add to dispatch loop (line ~280):**
```rust
while let Ok((mags, center_hz, rate)) = sdr_rx.try_recv() {
    eprintln!("[SDR] Received {} bins, center: {:.1} MHz", mags.len(), center_hz / 1e6);
    
    // Push to V-buffer
    let mut vb = vbuffer.lock();
    vb.push_frame(&gpu_shared.queue, &mags);
    
    // Log strong signals
    if mags.iter().any(|&m| m > 0.5) {
        eprintln!("[SDR] Strong signal at {:.1} MHz", center_hz / 1e6);
        
        // Create forensic event
        // (Add to forensic log)
    }
}
```

**Success:** Console shows `[SDR]` messages with signal data

---

### 2.2 Auto-Tune RTL-SDR (1 hour)

**Files:** `src/sdr.rs`

**Add frequency watchdog:**
```rust
// Watch detected audio frequency and tune RTL-SDR to match
let detected_freq = state.get_detected_freq();
let current_center = state.get_sdr_center_hz();

if (detected_freq - current_center).abs() > 5000.0 {
    // Retune RTL-SDR
    state.set_sdr_center_hz(detected_freq);
    eprintln!("[SDR] Auto-tuned to {:.1} kHz", detected_freq / 1e3);
}
```

**Success:** RTL-SDR follows audio harmonics automatically

---

## Phase 3: Missing Features (2-3 hours)

### 3.1 Active Denial/Masking (1-2 hours)

**Files:** Create `src/masking.rs`, modify `src/main.rs`

**Implementation:**
```rust
// Generate broadband noise at detected frequency
pub fn generate_masking_noise(center_hz: f32, bandwidth_hz: f32, duration_samples: usize) -> Vec<f32> {
    let mut rng = rand::thread_rng();
    (0..duration_samples)
        .map(|_| rng.gen_range(-1.0..1.0))
        .collect()
}
```

**UI Control:**
```slint
// Add to app.slint
property <bool> masking-active: false;
callback toggle_masking();
```

**Forensic Logging:**
```rust
// Log when masking activated
eprintln!("[MASKING] Activated at {:.1} MHz for {:.0} seconds", 
    center_hz / 1e6, duration_seconds);
```

**Success:** Can generate and output masking noise

---

### 3.2 Harmonic Tracker (1 hour)

**Files:** Create `src/harmonic.rs`

**Implementation:**
```rust
pub struct HarmonicTracker {
    fundamental_hz: f32,
    harmonics: Vec<f32>,
}

impl HarmonicTracker {
    pub fn new(fundamental_hz: f32) -> Self {
        Self {
            fundamental_hz,
            harmonics: (2..=10).map(|n| n as f32 * fundamental_hz).collect(),
        }
    }
    
    pub fn monitor(&self, spectrum: &[f32], sample_rate: f32) -> Vec<(usize, f32)> {
        self.harmonics.iter()
            .enumerate()
            .map(|(n, &freq)| {
                let bin = (freq / sample_rate * spectrum.len() as f32) as usize;
                (n + 2, spectrum.get(bin).copied().unwrap_or(0.0))
            })
            .filter(|(_, power)| *power > 0.1)
            .collect()
    }
}
```

**Success:** Displays harmonic power levels

---

## Phase 4: Testing & Validation (1 hour)

### 4.1 Verify All Fixes (30 min)

**Checklist:**
- [ ] `cargo build` shows <60 warnings
- [ ] UI shows waterfall updating
- [ ] UI shows spectrum bars
- [ ] Console shows `[Mamba] Loss: 0.xxxx`
- [ ] Console shows `[SDR] Received X bins`
- [ ] Forensic log has UTC timestamps
- [ ] Evidence report exports (HTML + CSV)

### 4.2 Document Known Issues (30 min)

**Create `docs/known_issues.md`:**
```markdown
# Known Issues

## Mamba Training
- Training may not converge with default threshold
- Workaround: Lower threshold to 10.0 dB for untrained models

## RTL-SDR
- Auto-tune may be too sensitive (retunes frequently)
- Workaround: Increase hysteresis to 10 kHz

## Performance
- Frame time may exceed 16ms during training
- Acceptable: 30-50ms average
```

---

## Time Budget Summary

| Phase | Task | Time |
|-------|------|------|
| 1.1 | Fix Mamba training | 1 hour |
| 1.2 | Wire waterfall | 30 min |
| 1.3 | Wire spectrum bars | 30 min |
| 1.4 | Fix 50 warnings | 1 hour |
| 2.1 | Wire RTL-SDR data | 1 hour |
| 2.2 | Auto-tune RTL-SDR | 1 hour |
| 3.1 | Active masking | 1-2 hours |
| 3.2 | Harmonic tracker | 1 hour |
| 4.1 | Verify fixes | 30 min |
| 4.2 | Document issues | 30 min |
| **Total** | | **6.5-7.5 hours** |

---

## Success Criteria

**After 6-8 hours:**
1. ✅ System compiles with <60 warnings
2. ✅ UI displays waterfall and spectrum
3. ✅ Mamba training shows non-zero loss
4. ✅ RTL-SDR data integrated and logged
5. ✅ Auto-tuning follows audio harmonics
6. ✅ Masking noise generation works
7. ✅ Harmonic tracker displays power levels
8. ✅ Forensic logs have UTC timestamps
9. ✅ Evidence reports export correctly
10. ✅ Known issues documented

**This is a full day's work but achievable.**

---

## If Time Runs Out

**Priority Order:**
1. Fix Mamba training (Phase 1.1)
2. Wire waterfall/spectrum (Phase 1.2-1.3)
3. Fix top 20 warnings (Phase 1.4 partial)
4. Wire RTL-SDR data (Phase 2.1)

**Skip If Short on Time:**
- Auto-tune RTL-SDR (Phase 2.2)
- Active masking (Phase 3.1)
- Harmonic tracker (Phase 3.2)

**Minimum Viable Product:**
- Compiles with <100 warnings
- UI displays waterfall
- Mamba training shows non-zero loss
- RTL-SDR data logged

---

**Document History:**
- 2025-03-06: Initial draft (6-8 hour implementation plan)
