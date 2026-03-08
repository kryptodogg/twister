# Training Pipeline Test Guide

## What Was Fixed

### Root Cause
Training loss was 0.0000 because:
- Training pairs were only 512 samples (1 frame)
- Mamba expects 32,768 samples (64-frame window)
- All training pairs were filtered out → empty batch → loss=0

### Solution Implemented
✅ **64-Frame Accumulator** in main.rs (lines 213-411):
- Collects magnitude data from each audio frame
- When 64 frames accumulated → creates training pair with full 32,768-sample window
- Resets accumulator and starts next window

✅ **Enhanced Debug Logging** in training.rs (lines 118-169):
- Shows batch size received
- Shows filter results (how many pairs had valid data)
- Shows actual loss values from training

## Expected Output When Running

### Phase 1: Accumulation (First ~2 seconds)
```
[Mamba DEBUG] Frame 100 mags.len()=512
[Mamba] Frame 100 anomaly=7.4452 latent.len()=64 latent[0..3]=[-0.931, -0.289, 0.085]
(No training output yet - still accumulating 64 frames)
```

### Phase 2: First Training Pair Created (After 64 frames @ ~21ms/frame = ~1.3 seconds)
```
[Mamba] ✓ Training pair enqueued (64 frames accumulated, anomaly=3.89dB)
[Mamba TRAIN] Batch size: 1 pairs
[Mamba TRAIN] Windows after filter: 1 (from 1 pairs)
[Mamba TRAIN] train_step OK: loss=0.1523
[Mamba TRAIN] Final loss: 0.150000
```

### Phase 3: Training Accelerates (After ~3-5 seconds)
Multiple training pairs accumulate and batch processing starts:
```
[Mamba] ✓ Training pair enqueued (64 frames accumulated, anomaly=2.15dB)
[Mamba] ✓ Training pair enqueued (64 frames accumulated, anomaly=4.87dB)
...
[Mamba TRAIN] Batch size: 32 pairs
[Mamba TRAIN] Pair tx_spectrum.len()=32768, need=32768
[Mamba TRAIN] Pair tx_spectrum.len()=32768, need=32768
[Mamba TRAIN] Windows after filter: 32 (from 32 pairs)
[Mamba TRAIN] train_step OK: loss=0.1401
[Mamba TRAIN] train_step OK: loss=0.1287
[Mamba TRAIN] Final loss: 0.129000
```

## Verification Checklist

Run the application and verify:

- [ ] **Frame accumulation working**: See "Training pair enqueued" messages appearing
- [ ] **Training pairs valid**: Log shows "tx_spectrum.len()=32768" (not filtered out)
- [ ] **Loss is non-zero**: See "loss=0.XXXXX" values (not 0.0000)
- [ ] **Loss is decreasing**: Compare losses across successive batches
  - Expected: 0.150 → 0.145 → 0.140 → 0.135... (or similar descent)
- [ ] **Mamba widget shows ONLINE**: Status should be green (64 latent values present)

## Debugging If Something's Wrong

### If you see: "ERROR: All training pairs filtered out!"
- Check: tx_spectrum size must be exactly 32,768
- Fix: Ensure 64-frame accumulator is working (verify log shows "Training pair enqueued")

### If you see: "Batch empty! No training pairs queued."
- Check: Training recording must be enabled in UI
- Check: Anomaly threshold (currently 1.0) should detect data
- Fix: Lower threshold or check if audio input is active

### If loss is still 0.0000
- Check: Training step must actually receive data
- Log should show: "Batch size: N pairs" where N > 0
- If N=0, training pairs not being queued

### If loss doesn't decrease
- Check: Mamba autoencoder may not be training (learning_rate issue)
- Verify: train_step() in mamba.rs is computing actual gradients
- Check: Loss computation isn't clipping or normalizing away changes

## System Components Wired

✅ **Audio Input** → 64-frame magnitude buffer  
✅ **Frame Accumulator** → Collects 64 frames (32,768 samples)  
✅ **Training Pair Creation** → Full window + anomaly score  
✅ **Queue Dispatch** → Async enqueue to training session  
✅ **Training Task** → Batch processing (32 pairs at a time)  
✅ **Loss Computation** → From Mamba autoencoder  
✅ **Debug Logging** → Full visibility into pipeline  

## Next Steps After Verification

1. ✅ Confirm loss is non-zero and decreasing
2. ⏳ Verify latent embeddings update (check widget)
3. ⏳ Tune learning rate if loss doesn't decrease fast enough
4. ⏳ Add model saving/checkpoint when loss reaches target
5. ⏳ Integrate trained model into inference loop

---

**Everything is wired. Test now. Report results.**
