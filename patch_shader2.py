import re

with open("src/visualization/shaders/gaussian_splatting.wgsl", "r") as f:
    content = f.read()

uniforms = """
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
"""

if "struct ForensicWeights" not in content:
    content = content.replace("var<workgroup> lds_keys: array<u32, BLOCK_SIZE>;", "var<workgroup> lds_keys: array<u32, BLOCK_SIZE>;\n" + uniforms)

# Replace the payload write step with intensity modification
# Scatter write
# dest_a = base_idx + lds_keys[tid * 2u];
write_step = """    if (dest_a < base_idx + BLOCK_SIZE) {
        out_payloads[dest_a] = lds_payloads[tid * 2u];
        out_keys[dest_a] = lds_keys[tid * 2u];
    }
    if (dest_b < base_idx + BLOCK_SIZE) {
        out_payloads[dest_b] = lds_payloads[(tid * 2u) + 1u];
        out_keys[dest_b] = lds_keys[(tid * 2u) + 1u];
    }"""

new_write_step = """    if (dest_a < base_idx + BLOCK_SIZE) {
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
    }"""

content = content.replace(write_step, new_write_step)

with open("src/visualization/shaders/gaussian_splatting.wgsl", "w") as f:
    f.write(content)
