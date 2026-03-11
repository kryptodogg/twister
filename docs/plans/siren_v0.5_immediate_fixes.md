# SIREN v0.5 - Immediate Fixes Plan

**Goal:** Make SIREN functional in 3-4 hours

**Date:** 2025-03-06  
**Status:** Actionable

---

## What's Actually Broken (Short List)

1. **Mamba training loss = 0.0000** - Not learning (1 hour)
2. **Waterfall not displayed** - Data computed but not shown in UI (30 min)
3. **Spectrum bars not displayed** - Same issue (30 min)
4. **112 compiler warnings** - Mostly unused variables (1 hour)
5. **RTL-SDR data not used** - Hardware works but data ignored (1 hour)

---

## Fix 1: Debug Mamba Training (1 hour)

**Symptom:** `loss=0.0000` every epoch

**Add Debug Logging:**

```rust
// src/training.rs - Line ~220 in spawn_background_training()
loop {
    if let Some(batch) = session.next_batch().await {
        eprintln!("[Mamba] Batch received: {} pairs", batch.len());  // ADD THIS
        let loss = trainer.step(&batch).await;
        eprintln!("[Mamba] Loss: {:.6}", loss);  // ADD THIS
        // ... rest of code
    }
}
```

```rust
// src/training.rs - Line ~120 in step()
let windows: Vec<Vec<f32>> = batch.iter()...
eprintln!("[Mamba] Windows extracted: {}", windows.len());  // ADD THIS
if windows.is_empty() {
    eprintln!("[Mamba] WARNING: Empty windows!");  // ADD THIS
    return 0.0;
}
```

**Expected Output:**
```
[Mamba] Batch received: 32 pairs
[Mamba] Windows extracted: 32
[Mamba] Loss: 0.1523
```

**If Output Shows:**
- `Batch received: 0 pairs` → Training pairs not being collected
- `Windows extracted: 0` → Pair format wrong
- `Loss: 0.0000` with non-zero windows → Gradient not flowing

**Fixes Based on Output:**

If pairs not collected, lower threshold in `src/main.rs`:
```rust
// Line ~365 - Change from -20.0 to 10.0 for untrained model
let threshold = 10.0;  // Was -20.0
if anomaly < threshold && !has_events {
    // collect pair
}
```

---

## Fix 2: Wire Waterfall Display (30 min)

**Add to timer callback in `src/main.rs` (line ~460):**

```rust
// Waterfall display
if let Ok(wf) = state.waterfall_rgba.lock() {
    let img = slint::Image::from_rgba8(
        slint::SharedPixelBuffer::clone_from_slice(
            wf.iter()
                .map(|&rgba| {
                    slint::Rgba8Pixel {
                        r: (rgba & 0xFF) as u8,
                        g: ((rgba >> 8) & 0xFF) as u8,
                        b: ((rgba >> 16) & 0xFF) as u8,
                        a: 255u8,
                    }
                })
                .collect::<Vec<_>>()
                .as_slice(),
            128, 64
        )
    );
    ui.set_waterfall_image(img);
}
```

**Verify in `ui/app.slint`:**
```slint
in-out property <image> waterfall-image;
// Should already exist - check line ~203
```

---

## Fix 3: Wire Spectrum Bars (30 min)

**Add to timer callback in `src/main.rs` (after waterfall code):**

```rust
// Spectrum bars display
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

**Verify in `ui/app.slint`:**
```slint
in-out property <string> spectrum-path-green;
// Should already exist - check line ~199
```

---

## Fix 4: Fix Top 20 Compiler Warnings (1 hour)

**Run and categorize:**
```bash
cargo build 2>&1 | grep "warning:" | head -20
```

**Quick Fixes:**

1. **Unused imports** - Delete the line:
```rust
// src/main.rs - Remove:
use crate::graph::ForensicGraph;  // DELETE
use crate::training::MambaTrainer;  // DELETE
use std::sync::Mutex;  // DELETE (if unused)
```

2. **Unused variables** - Prefix with `_`:
```rust
// src/mamba.rs - Line 115:
let (_t, _f) = x.dims2()?;  // Was (t, _f)
```

3. **Unnecessary `mut`** - Remove `mut`:
```rust
// src/main.rs - Lines 113, 115, 116, 118:
let gpu_ctx = GpuContext::new(...)?;  // Remove mut
let pdm = PdmEngine::new(...)?;  // Remove mut
let waterfall = WaterfallEngine::new(...)?;  // Remove mut
let bispec = BispectrumEngine::new(...)?;  // Remove mut
```

4. **Dead code** - Add `#[allow(dead_code)]` to structs you want to keep:
```rust
// src/anc.rs - Line 67:
#[allow(dead_code)]  // ADD THIS
pub struct AcousticTransfer {
    // ...
}
```

**Target:** Get from 112 warnings to <50 in 1 hour.

---

## Fix 5: RTL-SDR Data Integration (1 hour)

**Problem:** RTL-SDR runs but data not used.

**Quick Integration:**

```rust
// src/main.rs - Dispatch loop, after audio FFT (line ~280)
while let Ok((mags, center_hz, rate)) = sdr_rx.try_recv() {
    eprintln!("[SDR] Received {} bins, center: {:.1} MHz", mags.len(), center_hz / 1e6);
    
    // Push to V-buffer
    let mut vb = vbuffer.lock();
    vb.push_frame(&gpu_shared.queue, &mags);
    
    // Log RF detection
    if mags.iter().any(|&m| m > 0.5) {
        eprintln!("[SDR] Strong signal detected at {:.1} MHz", center_hz / 1e6);
    }
}
```

**Test:** Run and watch for `[SDR]` log messages.

---

## Verification Checklist

After all fixes, verify:

- [ ] `cargo build` shows <50 warnings (was 112)
- [ ] UI shows waterfall updating
- [ ] UI shows spectrum bars moving
- [ ] Console shows `[Mamba] Loss: 0.xxxx` (not 0.0000)
- [ ] Console shows `[SDR] Received X bins` messages
- [ ] Forensic log has UTC timestamps

---

## Time Budget

| Task | Time |
|------|------|
| Fix 1: Mamba training debug | 1 hour |
| Fix 2: Waterfall display | 30 min |
| Fix 3: Spectrum bars | 30 min |
| Fix 4: Compiler warnings | 1 hour |
| Fix 5: RTL-SDR integration | 1 hour |
| **Total** | **3.5 hours** |

---

## If You Get Stuck

**Mamba still not learning after 1 hour:**
- Skip it. System works without training (inference-only mode).
- Document: "Training pipeline under development"

**Waterfall/Spectrum not displaying:**
- Check Slint property names match exactly
- Verify image dimensions (128×64 for waterfall)

**Too many warnings:**
- Focus on `unused_mut` and `unused_imports` first (easiest)
- Add `#[allow(dead_code)]` to large unused structs

---

## Success Criteria

**After 3-4 hours:**
1. System compiles with <50 warnings
2. UI displays waterfall and spectrum
3. Mamba training shows non-zero loss (or documented as WIP)
4. RTL-SDR data logged to console
5. Forensic logs have UTC timestamps

**This is achievable in one work session.**

---

**Document History:**
- 2025-03-06: Initial draft (focused 3-4 hour plan)
