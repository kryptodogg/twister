struct CameraUniform {
    view_proj: mat4x4<f32>,
};
@group(0) @binding(0) var<uniform> camera: CameraUniform;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
};

@vertex
fn vs_main(
    @builtin(vertex_index) in_vertex_index: u32,
) -> VertexOutput {
    // Generate an infinite grid
    var positions = array<vec2<f32>, 4>(
        vec2<f32>(-100.0, -100.0),
        vec2<f32>( 100.0, -100.0),
        vec2<f32>(-100.0,  100.0),
        vec2<f32>( 100.0,  100.0),
    );
    let index = array<u32, 6>(0u, 1u, 2u, 1u, 3u, 2u)[in_vertex_index];
    let pos = positions[index];

    var out: VertexOutput;
    // Flat on Y=0 plane
    out.world_position = vec3<f32>(pos.x, 0.0, pos.y);
    out.clip_position = camera.view_proj * vec4<f32>(out.world_position, 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let world_pos = in.world_position;

    // Grid Lines
    let grid_size = 1.0;
    let line_width = 0.02;
    let grid_x = abs(fract(world_pos.x / grid_size + 0.5) - 0.5);
    let grid_z = abs(fract(world_pos.z / grid_size + 0.5) - 0.5);

    var color = vec4<f32>(0.0, 0.0, 0.0, 0.0);

    if (grid_x < line_width || grid_z < line_width) {
        color = vec4<f32>(0.0, 0.8, 1.0, 0.3); // Teal Grid
    }

    // Major Axes (N, S, E, W)
    if (abs(world_pos.x) < line_width * 2.0) {
        if (world_pos.z > 0.0) {
            color = vec4<f32>(1.0, 1.0, 1.0, 0.8); // North indicator (White Z+)
        } else {
            color = vec4<f32>(0.0, 0.5, 0.8, 0.5); // South indicator (Dark Teal Z-)
        }
    }
    if (abs(world_pos.z) < line_width * 2.0) {
        color = vec4<f32>(0.8, 0.2, 0.2, 0.5); // East/West indicator (Red X)
    }

    // Distance fading
    let dist = length(world_pos.xz);
    let fade = 1.0 - smoothstep(10.0, 50.0, dist);
    color.a *= fade;

    return color;
}
