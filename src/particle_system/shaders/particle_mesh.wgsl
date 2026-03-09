struct FrustumUniform {
    view_proj: mat4x4<f32>,
    particle_count: u32,
    _padding: vec3<u32>,
};

@group(0) @binding(0) var<uniform> frustum: FrustumUniform;
@group(0) @binding(1) var<storage, read> raw_particles: array<u32>;
@group(0) @binding(2) var<storage, read> visible_indices: array<u32>;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) properties: vec3<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32, @builtin(instance_index) instance_index: u32) -> VertexOutput {
    let particle_idx = visible_indices[instance_index];

    let offset = particle_idx * 11u;
    let px = bitcast<f32>(raw_particles[offset + 0u]);
    let py = bitcast<f32>(raw_particles[offset + 1u]);
    let pz = bitcast<f32>(raw_particles[offset + 2u]);

    let r = bitcast<f32>(raw_particles[offset + 3u]);
    let g = bitcast<f32>(raw_particles[offset + 4u]);
    let b = bitcast<f32>(raw_particles[offset + 5u]);
    let a = bitcast<f32>(raw_particles[offset + 6u]);

    let intensity = bitcast<f32>(raw_particles[offset + 7u]);
    let hardness = bitcast<f32>(raw_particles[offset + 8u]);
    let roughness = bitcast<f32>(raw_particles[offset + 9u]);
    let wetness = bitcast<f32>(raw_particles[offset + 10u]);

    let pos_world = vec3<f32>(px, py, pz);

    var local_pos = vec2<f32>(0.0);
    if (vertex_index == 0u) { local_pos = vec2<f32>(-1.0, -1.0); }
    else if (vertex_index == 1u) { local_pos = vec2<f32>(1.0, -1.0); }
    else if (vertex_index == 2u) { local_pos = vec2<f32>(-1.0, 1.0); }
    else if (vertex_index == 3u) { local_pos = vec2<f32>(1.0, -1.0); }
    else if (vertex_index == 4u) { local_pos = vec2<f32>(1.0, 1.0); }
    else if (vertex_index == 5u) { local_pos = vec2<f32>(-1.0, 1.0); }

    let scale = 0.05 * (1.0 + wetness);
    let final_pos = pos_world + vec3<f32>(local_pos.x * scale, local_pos.y * scale, 0.0);

    var out: VertexOutput;
    out.position = frustum.view_proj * vec4<f32>(final_pos, 1.0);
    out.color = vec4<f32>(r, g, b, a);
    out.uv = local_pos * 0.5 + 0.5;
    out.properties = vec3<f32>(intensity, hardness, roughness);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv_centered = in.uv - vec2<f32>(0.5);
    let dist = length(uv_centered);
    if (dist > 0.5) {
        discard;
    }

    let hardness_factor = max(0.01, in.properties.y);
    let alpha = smoothstep(0.5, 0.5 * (1.0 - hardness_factor), dist) * in.color.a * in.properties.x;
    return vec4<f32>(in.color.rgb, alpha);
}
