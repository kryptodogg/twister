enable f16;

struct VBufferPC {
    write_version: u32,
    context_len: u32,  // window size N (up to 512)
    freq_bins: u32,
    depth: u32,
}

@group(0) @binding(0) var<storage, read_write> vbuffer: array<vec4<f16>>;
var<push_constant> pc: VBufferPC;

const PI: f16 = 3.14159265359h;

fn vbuf_slot(version: u32, depth: u32) -> u32 {
    return version & (depth - 1u);
}

// Maximum window size for LDS. Since workgroup is 64 threads,
// and we want cooperative loading, we can load N frames per bin.
// Wait: The directive said: "Have the 64-thread wavefront cooperatively load the last N phase values from the GpuVBuffer into the LDS (Local Data Share), calculate the phase derivative (flux), and output the coherence mask directly into the 4th slot of the vec4<f16>."
// If 1 workgroup handles 1 bin, and has 64 threads, they can load up to 64 frames.
// N = 64 is the window size.

var<workgroup> lds_mag: array<f16, 64>;
var<workgroup> lds_phase: array<f16, 64>;

@compute @workgroup_size(64, 1, 1)
fn main(
    @builtin(workgroup_id) group_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>
) {
    let bin = group_id.x; // 1 workgroup per freq bin -> 512 workgroups total
    let tid = local_id.x; // 0..63 (maps to frame offset in history)
    let N = min(pc.context_len, 64u);

    // Each thread cooperatively loads 1 frame's data for this bin into LDS
    if (tid < N && pc.write_version >= tid) {
        let old_slot = vbuf_slot(pc.write_version - tid, pc.depth);
        let old_offset = old_slot * pc.freq_bins + bin;
        let data = vbuffer[old_offset];
        lds_mag[tid] = data.x;
        lds_phase[tid] = data.y;
    } else {
        lds_mag[tid] = 0.0h;
        lds_phase[tid] = 0.0h;
    }

    workgroupBarrier();

    // Only thread 0 computes the metrics for this bin and writes it back
    if (tid == 0u) {
        var sum_mag_diff: f16 = 0.0h;
        var sum_mag: f16 = lds_mag[0];

        var sum_phase_diff: f16 = 0.0h;
        var sum_phase_diff_sq: f16 = 0.0h;

        let valid_N = min(N, pc.write_version + 1u);
        var count: f16 = f16(valid_N - 1u);

        if (valid_N > 1u) {
            for (var i = 1u; i < valid_N; i++) {
                let mag = lds_mag[i];
                let prev_mag = lds_mag[i - 1u];

                let phase = lds_phase[i];
                let prev_phase = lds_phase[i - 1u];

                // Mag flux
                sum_mag_diff += abs(prev_mag - mag);
                sum_mag += mag;

                // Phase diff
                var diff = abs(prev_phase - phase);
                while (diff > 2.0h * PI) { diff -= 2.0h * PI; }
                if (diff > PI) { diff = 2.0h * PI - diff; }

                sum_phase_diff += diff;
                sum_phase_diff_sq += diff * diff;
            }
        }

        var flux: f16 = 0.0h;
        var coherence: f16 = 0.0h;

        if (valid_N > 1u && sum_mag > 0.0h) {
            let mean_diff = sum_mag_diff / count;
            let mean_mag = sum_mag / f16(valid_N);
            flux = clamp(mean_diff / mean_mag, 0.0h, 1.0h);
        }

        if (valid_N > 1u) {
            let mean_pdiff = sum_phase_diff / count;
            let mean_psq = sum_phase_diff_sq / count;
            let variance = mean_psq - mean_pdiff * mean_pdiff;
            coherence = clamp(1.0h - variance / (PI * PI), 0.0h, 1.0h);
        }

        // Write to current slot (which is tid=0)
        let current_slot = vbuf_slot(pc.write_version, pc.depth);
        let current_offset = current_slot * pc.freq_bins + bin;
        vbuffer[current_offset] = vec4<f16>(lds_mag[0], lds_phase[0], flux, coherence);
    }
}
