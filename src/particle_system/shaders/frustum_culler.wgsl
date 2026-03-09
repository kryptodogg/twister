struct FrustumUniform {
    view_proj: mat4x4<f32>,
    particle_count: u32,
    _padding: vec3<u32>,
};

@group(0) @binding(0) var<uniform> frustum: FrustumUniform;

@group(0) @binding(1) var<storage, read> raw_particles: array<u32>;

struct DrawIndirectArgs {
    vertex_count: u32,
    instance_count: atomic<u32>,
    base_vertex: u32,
    base_instance: u32,
};
@group(0) @binding(2) var<storage, read_write> draw_indirect: DrawIndirectArgs;
@group(0) @binding(3) var<storage, read_write> visible_indices: array<u32>;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if (idx >= frustum.particle_count) {
        return;
    }

    let offset = idx * 11u;
    let px = bitcast<f32>(raw_particles[offset + 0u]);
    let py = bitcast<f32>(raw_particles[offset + 1u]);
    let pz = bitcast<f32>(raw_particles[offset + 2u]);

    let pos_world = vec4<f32>(px, py, pz, 1.0);
    let clip_space = frustum.view_proj * pos_world;

    let w = clip_space.w;

    if (clip_space.x >= -w && clip_space.x <= w &&
        clip_space.y >= -w && clip_space.y <= w &&
        clip_space.z >= 0.0 && clip_space.z <= w) {

        let visible_idx = atomicAdd(&draw_indirect.instance_count, 1u);
        visible_indices[visible_idx] = idx;
    }
}
