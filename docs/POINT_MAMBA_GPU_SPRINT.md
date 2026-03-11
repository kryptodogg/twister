# Point Mamba GPU-Optimized Sprint (Revised with f16 + LDS)

**Doctrine Enforced**: Wave64 + f16 half-precision + vec4 vectorized fetching + LDS-backed Blelloch Scan
**Target Performance**: 169 fps on RX 6700 XT (5.9ms latency)
**Memory Model**: Zero padding, dense packed data via f16 + LDS cooperative streaming
**Branch**: task-c-point-mamba
**Start Date**: 2026-03-08

---

## GPU Architecture: f16 + Vectorized Fetching + LDS Streaming

### Key Insights

**1. f16 Memory Revolution (50% Footprint Reduction)**
- wgpu::Features::SHADER_F16 enables half-precision floats
- Memory savings: 256-byte VRAM fetch carries 512 bytes of semantic data
- No dead padding needed: tightly-packed f16 arrays are hardware-native

**2. Vectorized Fetching (128-Bit Native Atomicity)**
- Group spatial attributes into `vec4<f16>` bundles
- Instead of separate azimuth[], elevation[], frequency[], threat_level[] arrays
- Bundle as single `array<vec4<f16>, N>` with [azimuth, elevation, frequency, threat_level] per element
- RDNA2 fetches vec4 in single 128-bit atomic transaction (no sub-channel masking)

**3. LDS-Backed Cooperative Streaming (TB/s Throughput)**
- Define `var<workgroup> lds_buffer: array<vec4<f16>, 256>` (shared memory within Workgroup Processor)
- All 64 threads cooperatively load global VRAM → LDS in parallel (1 load per thread)
- Call `workgroupBarrier()` to synchronize
- Once in LDS, all threads access shared data at direct ALU speeds (no VRAM latency)
- Blelloch Scan for cumulative reductions (parallel prefix sum)

### WGSL Shader Structure (Function-Packed Selective Scan)

```wgsl
enable f16;  // Enable half-precision float support

@compute @workgroup_size(64, 1, 1)
@subgroup_size(64)
fn selective_scan_packed(
    @builtin(global_invocation_id) gid: vec3<u32>,
    @builtin(local_invocation_id) lid: vec3<u32>,
    @builtin(workgroup_id) wgid: vec3<u32>,

    @binding(0) points: binding_array<vec4<f16>>,  // Packed [azimuth, elev, freq, threat]
    @binding(1) A: storage_buffer<array<vec4<f16>>>,  // (128/4 = 32 vec4s)
    @binding(2) B: storage_buffer<array<vec4<f16>>>,
    @binding(3) out_h: storage_buffer<array<vec4<f16>>>  // Output state
) {
    // ===== LDS Cooperative Streaming =====
    var<workgroup> lds: array<vec4<f16>, 256>;  // Shared memory

    // All 64 threads load 4 consecutive vec4s each (64 * 4 = 256)
    let tid = lid.x;
    for (var i = 0u; i < 4u; i += 1u) {
        let gid_idx = wgid.x * 256u + tid * 4u + i;
        lds[tid * 4u + i] = points[gid_idx];
    }
    workgroupBarrier();  // Wait for all loads to complete

    // ===== Function-Packed Selective Scan =====
    // All 64 threads compute identical path (zero divergence)
    var h_packed: vec4<f16> = vec4<f16>(0.0);  // 4×f16 state (64 bits)

    for (var t = 0u; t < 8u; t += 1u) {
        let u_t = lds[tid];  // Load from LDS (fast)
        let delta = select(0.0h, 1.0h, u_t.z > 0.0);  // Gating: no branching

        // State evolution packed into single op:
        // h = A @ h + (delta * u) ⊙ B
        let h_new = matmul_f16_packed(A, h_packed, tid) + (delta * u_t) * B[tid];
        h_packed = h_new;

        workgroupBarrier();  // Warp sync (zero overhead in LDS)
    }

    // Readout: y = C ⊙ h (element-wise, no branching)
    out_h[gid.x] = h_packed;
}

// Function-packed 4×4 matrix multiply (fits in 32 VGPRs)
fn matmul_f16_packed(A: storage_buffer<array<vec4<f16>>>, h: vec4<f16>, tid: u32) -> vec4<f16> {
    var result: vec4<f16> = vec4<f16>(0.0);
    for (var i = 0u; i < 32u; i += 1u) {
        result += A[tid * 32u + i] * h[i % 4u];
    }
    return result;
}
```

---

## Implementation Phase Breakdown

### Phase 3A: PointNet Encoder (Mon 3/8, 3 hours) ✅
- ✅ Function-packed MLP1 (6 → 64), MLP2 (64 → 128), MLP3 (128 → 256)
- ✅ Tightly-scoped intermediate tensors (freed immediately)
- ✅ 10 tests passing
- **Status**: COMPLETE

### Phase 3B: PointMamba Blocks (Tue-Wed 3/9-3/10, 6 hours) 🔄
- ✅ MambaBlock with function-packed selective scan
- ✅ Residual connection for gradient flow
- ✅ 5 tests passing
- **Next**: Wire to f16 WGSL kernel with LDS streaming
- **GPU Optimization**:
  - ✅ VGPR <32 (h_packed is vec4<f16> = 64 bits)
  - ✅ Zero divergence (all threads compute identical path)
  - ✅ Workgroup sync via `workgroupBarrier()`
  - 🔄 TODO: Implement LDS-backed cooperative load

### Phase 3C: Point Decoder (Wed 3/10, 2 hours)
- File: `src/ml/point_decoder.rs` (150 lines)
- Input: (N, 128) point features
- Output: (N, 3) 3D offset [Δx, Δy, Δz]
- Function-packed MLP: (128) → (256) → (128) → (3)
- **Tests**: 8 tests covering shape, bounds, numerical stability

### Phase 3D: Gaussian Splatting Renderer with f16 + Vectorized Fetching (Thu 3/11, 4 hours)
- File: `src/visualization/gaussian_splatting_f16.rs` (450 lines)
- **WGSL Kernel**:
  ```wgsl
  @compute @workgroup_size(16, 16, 1)
  fn gaussian_splat_f16(...) {
      // Vectorized fetching: read vec4<f16> point bundle per iteration
      // LDS-backed accumulation: splat values into workgroup lds[]
      // Blelloch Scan for reduction (max intensity per voxel)
      // Output tonemap as vec4<u32> (RGBA)
  }
  ```
- **Memory Model**:
  - Input: vec4<f16> point bundles (16 bytes = 4×f16)
  - LDS: var<workgroup> accumulator[256] (vec4<f16>)
  - Output: 1024×1024 RGBA texture (4 MB)
- **Performance**:
  - Point iteration: 64 threads × 4 vec4 loads = 256 points processed in parallel
  - LDS accumulation: zero VRAM latency
  - Target: > 160 fps (< 2.5ms kernel execution)

### Phase 3E: Trainer + Integration (Fri 3/12, 3 hours)
- File: `src/ml/point_mamba_trainer.rs` (300 lines)
- Load point cloud corpus, extract point clouds (N, 6)
- **Training Objectives**:
  1. Wavefield reconstruction: MSE(Points_reconstructed, Points_input)
  2. Temporal stability: L1(Δt - Δt_prev)
  3. Sparsity: L1(||Δx|| + ||Δy|| + ||Δz||)
- **Loss Function**: λ1 * MSE + λ2 * temporal + λ3 * sparsity
- **Expected Convergence**: 2.1 → 0.45 loss in 30 epochs

---

## GPU Optimization Verification Checklist

**Before merging Phase 3D (Gaussian Splatting with f16):**

- [ ] **f16 Feature Enabled**: wgpu::Features::SHADER_F16 requested in device creation
- [ ] **Vectorized Fetching**: All point attributes bundled as `vec4<f16>`
- [ ] **LDS Cooperative Load**: All 64 threads load 4 vec4s each → LDS workgroup buffer
- [ ] **Zero Divergence**: All threads compute identical path (use select() instead of if)
- [ ] **VGPR Pressure**: Function-packed state fits in <32 VGPRs (h_packed is single vec4<f16>)
- [ ] **Workgroup Sync**: Only `workgroupBarrier()` calls, no shared memory spins
- [ ] **Performance Target**: > 160 fps (< 2.5ms for 1024×1024 viewport)
- [ ] **Memory Footprint**: Total VRAM < 4.5GB (input data + LDS + output texture)
- [ ] **All 38 Tests Passing**: PointNet(10) + PointMamba(5) + Decoder(8) + Splatting(8) + Trainer(8)

---

## Key Architectural Decisions

### Why f16 Over f32?
- Point cloud spatial coordinates (azimuth, elevation) naturally fit in f16 (±180°)
- Frequency can be log-scaled to f16 range (1 Hz - 1 GHz → 10 bits)
- State vectors (h_t) benefit from f16 quantization (neural networks robust to precision loss)
- **Result**: 2× data density, same memory bandwidth

### Why vec4 Bundling?
- RDNA2 has optimized 128-bit load/store units
- Requesting vec4 is atomic at hardware level (no sub-channel masking)
- Natural stride alignment: 4 × f16 = 64 bits, 16 elements per 256-byte burst
- **Result**: Zero padding overhead, native coalescing

### Why LDS Streaming?
- Global VRAM access: 200+ cycle latency per miss
- LDS access: Direct ALU, 0-cycle latency for registered data
- Cooperative load: All 64 threads pull data in parallel, then synchronize
- **Result**: TB/s throughput after initial load synchronization

---

## Expected Timeline

| Phase | Task | Duration | GPU Arch | Status |
|-------|------|----------|----------|--------|
| 3A | PointNet Encoder | 3h | Function-packing | ✅ Complete |
| 3B | PointMamba Blocks | 6h | f16 + LDS (WIP) | 🔄 In Progress |
| 3C | Point Decoder | 2h | Function-packing | Pending |
| 3D | Gaussian Splatting f16 | 4h | f16 + vec4 + LDS | Pending |
| 3E | Trainer Integration | 3h | Standard Burn ML | Pending |
| **Total** | **Phase 3 Complete** | **~20h** | **Advanced GPU** | **🔄 On Track** |

---

## Files to Create/Modify

**Already Created**:
- ✅ `src/ml/pointnet_encoder.rs` (250 lines, 10 tests)
- ✅ `src/ml/mamba_block.rs` (100 lines, 5 tests)

**To Create**:
- `src/ml/point_decoder.rs` (150 lines, 8 tests)
- `src/visualization/gaussian_splatting_f16.rs` (450 lines, WGSL + Rust)
- `src/ml/point_mamba_trainer.rs` (300 lines, 8 tests)
- `src/visualization/shaders/selective_scan_f16.wgsl` (120 lines, LDS streaming)

---

## Critical Success Metrics

✅ **Phase 3A (PointNet)**: Input shape (N, 6) → output (N, 256), 10/10 tests pass
✅ **Phase 3B (MambaBlock)**: Selective scan with residual, 5/5 tests pass
🔄 **Phase 3C (Decoder)**: (N, 128) → (N, 3), 8/8 tests pass
🔄 **Phase 3D (Splatting)**: 169 fps on RX 6700 XT, f16 + LDS verified
🔄 **Phase 3E (Trainer)**: Loss converges 2.1 → 0.45, training stable

**Go/No-Go Decision**: All phases must show no NaNs, all gradients flowing, performance > 160 fps

---

## References

- `docs/GPU_OPTIMIZATION_DOCTRINE.md` - Three Pillars (register pressure, thread divergence, subgroup ops)
- `memory/GPU_OPTIMIZATION_PRINCIPLES.md` - Locked baseline (Wave64 + 256-byte, 4.0x over Wave32)
- User Guidance: f16 + LDS cooperative streaming (bypass padding, maximize silicon density)
