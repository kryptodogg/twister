enable f16;

struct VBufferPC {
    write_version: u32,
    context_len: u32,
    freq_bins: u32,
    depth: u32,
}

// raw IQ samples: 512 complex samples = 512 * 2 bytes = 1024 bytes
// 1024 bytes / 4 bytes per u32 = 256 u32 elements.
// Each u32 holds 4 i8s, which is 2 IQ samples (I, Q, I, Q).
@group(0) @binding(0) var<storage, read> raw_iq: array<u32>;
@group(0) @binding(1) var<storage, read_write> vbuffer: array<vec4<f16>>;
var<push_constant> pc: VBufferPC;

var<workgroup> lds_complex: array<vec2<f16>, 512>;

const PI: f16 = 3.14159265359h;

fn extract_i8(word: u32, offset: u32) -> f16 {
    // extract 8-bit segment and sign-extend
    let shift = offset * 8u;
    let b = (word >> shift) & 0xFFu;
    var i = i32(b);
    if ((i & 0x80) != 0) {
        i = i - 256;
    }
    return f16(i);
}

@compute @workgroup_size(64, 1, 1)
fn main(
    @builtin(local_invocation_id) local_id: vec3<u32>
) {
    let tid = local_id.x; // 0..63

    // Step A: Load, extract IQ, apply bit-reversal
    for (var i = 0u; i < 8u; i++) {
        let logical_idx = tid + i * 64u; // 0..511
        let u32_idx = logical_idx / 2u;
        let is_upper = (logical_idx % 2u) != 0u;

        let word = raw_iq[u32_idx];
        var i_val: f16 = 0.0h;
        var q_val: f16 = 0.0h;

        if (is_upper) {
            i_val = extract_i8(word, 2u);
            q_val = extract_i8(word, 3u);
        } else {
            i_val = extract_i8(word, 0u);
            q_val = extract_i8(word, 1u);
        }

        // Bit reversal (9 bits)
        var rev = 0u;
        var x = logical_idx;
        for (var bit = 0u; bit < 9u; bit++) {
            rev = (rev << 1u) | (x & 1u);
            x = x >> 1u;
        }

        lds_complex[rev] = vec2<f16>(i_val, q_val);
    }

    workgroupBarrier();

    for (var stage = 1u; stage <= 9u; stage++) {
        let m = 1u << stage;
        let m2 = m >> 1u;

        for (var k = 0u; k < 4u; k++) {
            let butterfly_idx = tid + k * 64u;
            let group = butterfly_idx / m2;
            let offset = butterfly_idx % m2;

            let i_idx = group * m + offset;
            let j_idx = i_idx + m2;

            let angle = f16(offset) * PI / f16(m2);
            let w_real = cos(angle);
            let w_imag = -sin(angle);

            let t = lds_complex[j_idx];
            let u = lds_complex[i_idx];

            let t_real = t.x * w_real - t.y * w_imag;
            let t_imag = t.x * w_imag + t.y * w_real;
            let t_w = vec2<f16>(t_real, t_imag);

            lds_complex[i_idx] = vec2<f16>(u.x + t_w.x, u.y + t_w.y);
            lds_complex[j_idx] = vec2<f16>(u.x - t_w.x, u.y - t_w.y);
        }

        workgroupBarrier();
    }

    // Step C & D: Extract Mag/Phase and write to VRAM
    let slot = pc.write_version & (pc.depth - 1u);
    let base_offset = slot * pc.freq_bins;

    for (var i = 0u; i < 8u; i++) {
        let logical_idx = tid + i * 64u;
        let c = lds_complex[logical_idx];

        let mag = length(c);
        let phase = atan2(c.y, c.x); // returns phase in [-PI, PI]

        vbuffer[base_offset + logical_idx] = vec4<f16>(mag, phase, 0.0h, 0.0h);
    }
}
