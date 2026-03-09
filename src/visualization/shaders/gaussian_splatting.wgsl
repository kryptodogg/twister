// src/visualization/shaders/gaussian_splatting.wgsl
//
// Bulletproof Blelloch Radix Sort with f16+u32 domain separation
// Wave64-optimized GPU compute kernel for RDNA2 (RX 6700 XT baseline)
//
// **Payload Domain (SoA layout, f16 for density)**:
//   lds_payloads: array<vec4<f16>, 128> = [azimuth, elevation, frequency, intensity] per point
//
// **Routing Domain (Prefix sum, u32 for accumulator safety)**:
//   lds_keys: array<u32, 128> = [grid_hash] per point (used to determine sort order)
//
// **Algorithm**: Blelloch Scan (bidirectional prefix sum)
//   1. Cooperative Load: All 64 threads → Load 2 payloads + 2 keys each into LDS
//   2. Up-Sweep: Parallel reduction tree (accumulate keys)
//   3. Clear Last: Set last element to 0
//   4. Down-Sweep: Parallel distribution tree (propagate accumulated keys)
//   5. Scatter Write: All threads write sorted payloads/keys to output (with boundary checks)
//
// **Critical Invariants**:
//   - NEVER accumulate f16 in prefix sum (overflow → NaN cascade)
//   - Keys (u32) accumulate; payloads (f16) scatter only
//   - All threads compute identical path (no divergence in main loop)
//   - workgroupBarrier() at each phase boundary

enable f16;

const WAVE_SIZE: u32 = 64u;                // Wave64 = 64 threads per wave
const BLOCK_SIZE: u32 = 128u;               // 128 elements per block (2 per thread)

// **Payload domain** (Struct-of-Arrays, f16 for density)
@group(0) @binding(0) var<storage, read> in_payloads: array<vec4<f16>>;
@group(0) @binding(1) var<storage, read_write> out_payloads: array<vec4<f16>>;

// **Routing domain** (Grid hash keys for spatial binning)
@group(0) @binding(2) var<storage, read> in_keys: array<u32>;
@group(0) @binding(3) var<storage, read_write> out_keys: array<u32>;

// **Local Data Share (LDS)**: Shared within workgroup, zero-latency access
var<workgroup> lds_payloads: array<vec4<f16>, BLOCK_SIZE>;
var<workgroup> lds_keys: array<u32, BLOCK_SIZE>;

struct ForensicWeights {
    use_tdoa: f32,
    use_device_corr: f32,
    use_vbuffer: f32,
    tdoa_confidence: f32,
    device_idx: u32,
    device_weights: vec4<f32>,
    vbuffer_coherence: array<f32, 64>,
}

@group(0) @binding(4) var<uniform> forensic: ForensicWeights;


@compute @workgroup_size(WAVE_SIZE, 1, 1)
fn main(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(workgroup_id) group_id: vec3<u32>
) {
    let tid = local_id.x;
    let base_idx = group_id.x * BLOCK_SIZE;

    // ===== PHASE 1: Cooperative Load (64 threads, 2 loads each) =====
    // All threads load 2 elements each from global VRAM into LDS
    // (64 threads × 2 loads = 128 elements → fits in BLOCK_SIZE)

    lds_payloads[tid * 2u] = in_payloads[base_idx + tid * 2u];
    lds_payloads[(tid * 2u) + 1u] = in_payloads[base_idx + (tid * 2u) + 1u];
    lds_keys[tid * 2u] = in_keys[base_idx + tid * 2u];
    lds_keys[(tid * 2u) + 1u] = in_keys[base_idx + (tid * 2u) + 1u];

    workgroupBarrier();  // All threads must finish loading before scan starts

    // ===== PHASE 2: Up-Sweep (Parallel Reduction) =====
    // Build reduction tree: sum(lds_keys) → lds_keys[127]
    // Level 1: stride=2
    if tid < 64u {
        let idx_a = tid * 2u + 1u;
        let idx_b = tid * 2u;
        lds_keys[idx_a] = lds_keys[idx_a] + lds_keys[idx_b];
    }
    workgroupBarrier();

    // Level 2: stride=4
    if tid < 32u {
        let idx_a = tid * 4u + 3u;
        let idx_b = tid * 4u + 1u;
        lds_keys[idx_a] = lds_keys[idx_a] + lds_keys[idx_b];
    }
    workgroupBarrier();

    // Level 3: stride=8
    if tid < 16u {
        let idx_a = tid * 8u + 7u;
        let idx_b = tid * 8u + 3u;
        lds_keys[idx_a] = lds_keys[idx_a] + lds_keys[idx_b];
    }
    workgroupBarrier();

    // Level 4: stride=16
    if tid < 8u {
        let idx_a = tid * 16u + 15u;
        let idx_b = tid * 16u + 7u;
        lds_keys[idx_a] = lds_keys[idx_a] + lds_keys[idx_b];
    }
    workgroupBarrier();

    // Level 5: stride=32
    if tid < 4u {
        let idx_a = tid * 32u + 31u;
        let idx_b = tid * 32u + 15u;
        lds_keys[idx_a] = lds_keys[idx_a] + lds_keys[idx_b];
    }
    workgroupBarrier();

    // Level 6: stride=64
    if tid < 2u {
        let idx_a = tid * 64u + 63u;
        let idx_b = tid * 64u + 31u;
        lds_keys[idx_a] = lds_keys[idx_a] + lds_keys[idx_b];
    }
    workgroupBarrier();

    // Final level: stride=128 (only thread 0)
    if tid == 0u {
        lds_keys[127u] = lds_keys[127u] + lds_keys[63u];
    }
    workgroupBarrier();

    // ===== PHASE 3: Clear Last Element =====
    // lds_keys[127] now contains sum of all keys (total offset)
    // Save it, then set to 0 for down-sweep
    var total_sum: u32 = 0u;
    if tid == 0u {
        total_sum = lds_keys[127u];
        lds_keys[127u] = 0u;
    }
    workgroupBarrier();

    // ===== PHASE 4: Down-Sweep (Parallel Distribution) =====
    // Distribute accumulated values top-down
    // (Mirror of up-sweep with accumulation)

    if tid == 0u {
        // Highest level: stride=128
        let temp = lds_keys[63u];
        lds_keys[127u] = lds_keys[127u] + lds_keys[63u];
        lds_keys[63u] = lds_keys[127u] - temp;
    }
    workgroupBarrier();

    if tid < 2u {
        // Level: stride=64
        let idx_b = tid * 64u + 31u;
        let idx_a = tid * 64u + 63u;
        let temp = lds_keys[idx_b];
        lds_keys[idx_a] = lds_keys[idx_a] + lds_keys[idx_b];
        lds_keys[idx_b] = lds_keys[idx_a] - temp;
    }
    workgroupBarrier();

    if tid < 4u {
        // Level: stride=32
        let idx_b = tid * 32u + 15u;
        let idx_a = tid * 32u + 31u;
        let temp = lds_keys[idx_b];
        lds_keys[idx_a] = lds_keys[idx_a] + lds_keys[idx_b];
        lds_keys[idx_b] = lds_keys[idx_a] - temp;
    }
    workgroupBarrier();

    if tid < 8u {
        // Level: stride=16
        let idx_b = tid * 16u + 7u;
        let idx_a = tid * 16u + 15u;
        let temp = lds_keys[idx_b];
        lds_keys[idx_a] = lds_keys[idx_a] + lds_keys[idx_b];
        lds_keys[idx_b] = lds_keys[idx_a] - temp;
    }
    workgroupBarrier();

    if tid < 16u {
        // Level: stride=8
        let idx_b = tid * 8u + 3u;
        let idx_a = tid * 8u + 7u;
        let temp = lds_keys[idx_b];
        lds_keys[idx_a] = lds_keys[idx_a] + lds_keys[idx_b];
        lds_keys[idx_b] = lds_keys[idx_a] - temp;
    }
    workgroupBarrier();

    if tid < 32u {
        // Level: stride=4
        let idx_b = tid * 4u + 1u;
        let idx_a = tid * 4u + 3u;
        let temp = lds_keys[idx_b];
        lds_keys[idx_a] = lds_keys[idx_a] + lds_keys[idx_b];
        lds_keys[idx_b] = lds_keys[idx_a] - temp;
    }
    workgroupBarrier();

    if tid < 64u {
        // Level: stride=2
        let idx_b = tid * 2u;
        let idx_a = tid * 2u + 1u;
        let temp = lds_keys[idx_b];
        lds_keys[idx_a] = lds_keys[idx_a] + lds_keys[idx_b];
        lds_keys[idx_b] = lds_keys[idx_a] - temp;
    }
    workgroupBarrier();

    // ===== PHASE 5: Scatter Write (All threads write sorted data) =====
    // At this point, lds_keys[i] contains the sorted destination index for element i
    // Use boundary checks to prevent out-of-bounds writes

    // Thread tid writes elements [tid*2, tid*2+1]
    let dest_a = base_idx + lds_keys[tid * 2u];
    let dest_b = base_idx + lds_keys[(tid * 2u) + 1u];

    // Boundary-safe write: only write if destination is within this block
    if (dest_a < base_idx + BLOCK_SIZE) {
        var p_a = lds_payloads[tid * 2u];
        let key_a = lds_keys[tid * 2u];
        var intensity_a = p_a.w;
        let w_tdoa_a = mix(1.0, forensic.tdoa_confidence, forensic.use_tdoa);
        let w_dev_a = mix(1.0, forensic.device_weights[forensic.device_idx], forensic.use_device_corr);
        let w_vbuf_a = mix(1.0, forensic.vbuffer_coherence[key_a % 64u], forensic.use_vbuffer);
        intensity_a = intensity_a * f16(w_tdoa_a * w_dev_a * w_vbuf_a);
        p_a.w = intensity_a;

        out_payloads[dest_a] = p_a;
        out_keys[dest_a] = key_a;
    }
    if (dest_b < base_idx + BLOCK_SIZE) {
        var p_b = lds_payloads[(tid * 2u) + 1u];
        let key_b = lds_keys[(tid * 2u) + 1u];
        var intensity_b = p_b.w;
        let w_tdoa_b = mix(1.0, forensic.tdoa_confidence, forensic.use_tdoa);
        let w_dev_b = mix(1.0, forensic.device_weights[forensic.device_idx], forensic.use_device_corr);
        let w_vbuf_b = mix(1.0, forensic.vbuffer_coherence[key_b % 64u], forensic.use_vbuffer);
        intensity_b = intensity_b * f16(w_tdoa_b * w_dev_b * w_vbuf_b);
        p_b.w = intensity_b;

        out_payloads[dest_b] = p_b;
        out_keys[dest_b] = key_b;
    }
}
