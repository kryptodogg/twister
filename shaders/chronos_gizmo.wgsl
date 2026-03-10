struct CameraUniform {
    view_proj: mat4x4<f32>,
};
@group(0) @binding(0) var<uniform> camera: CameraUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    // Scale and position the gizmo in world space (bottom left corner conceptually)
    let scaled_pos = model.position * 0.1;
    let world_pos = vec3<f32>(-0.8, -0.8, -2.0) + scaled_pos; // Fixed relative to camera
    out.clip_position = camera.view_proj * vec4<f32>(world_pos, 1.0);
    out.color = model.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0); // Solid color for axes
}
