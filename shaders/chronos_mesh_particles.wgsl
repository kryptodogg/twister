struct CameraUniform {
    view_proj: mat4x4<f32>,
    time: f32,
    mode: u32, // 0 = Live, 1 = Scrub/Review
    scrub_progress: f32, // 0.0 to 1.0
};
@group(0) @binding(0) var<uniform> camera: CameraUniform;

struct Particle {
    position: vec3<f32>,
    _pad1: f32,
    color: vec4<f32>,
    size: f32,
    timestamp: f32, // 0.0 to 1.0 mapping across the 97 days
    _pad2: vec2<f32>,
};

@group(1) @binding(0) var<storage, read> particles: array<Particle>;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
) -> VertexOutput {
    let particle = particles[instance_index];

    // Visibility logic for DVR
    var is_visible = true;
    if (camera.mode == 1u) {
        // Scrubbing/Review: Only show particles near the scrubbed timestamp
        let time_window = 0.02; // Show a small slice of time
        if (abs(particle.timestamp - camera.scrub_progress) > time_window) {
            is_visible = false;
        }
    } else {
        // Live mode: Show the most recent particles (simulated here as highest timestamps)
        if (particle.timestamp < 0.95) {
             is_visible = false;
        }
    }

    // Quick discard hack (degenerate quad)
    var size = particle.size;
    if (!is_visible) {
       size = 0.0;
    }

    var pos = array<vec2<f32>, 4>(
        vec2<f32>(-0.5, -0.5),
        vec2<f32>( 0.5, -0.5),
        vec2<f32>(-0.5,  0.5),
        vec2<f32>( 0.5,  0.5),
    );
    let index = array<u32, 6>(0u, 1u, 2u, 1u, 3u, 2u)[vertex_index];
    let offset = pos[index] * size;

    var out: VertexOutput;

    // Simple billboard facing +Z for demonstration
    let world_pos = particle.position + vec3<f32>(offset.x, offset.y, 0.0);

    out.clip_position = camera.view_proj * vec4<f32>(world_pos, 1.0);
    out.color = particle.color;

    // Live pulse
    if (camera.mode == 0u && is_visible) {
        let pulse = 0.5 + 0.5 * sin(camera.time * 10.0 + f32(instance_index) * 0.1);
        out.color.a *= pulse;
    }

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
