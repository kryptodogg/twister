// src/visualization/shaders/mesh_lod_task.wgsl
// Task Shader: Per-instance LOD calculation for adaptive mesh rendering
//
// This shader calculates the appropriate LOD level for each attack source
// based on its projected screen coverage in pixels.
//
// Input: Camera parameters, instance positions and intensities
// Output: LOD selection stored in indirect draw buffer

struct CameraUniform {
    view_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    viewport_width: u32,
    viewport_height: u32,
}

struct InstanceData {
    position: vec3<f32>,
    intensity: f32,
}

struct LodPayload {
    lod_level: u32,
    vertex_count: u32,
    triangle_count: u32,
    instance_id: u32,
}

// Bind group 0: Camera and instance data
@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(0) @binding(1) var<storage, read> instances: array<InstanceData>;
@group(0) @binding(2) var<storage, read_write> lod_decisions: array<LodPayload>;

// LOD level thresholds (screen coverage in pixels)
const LOD_THRESHOLD_HIGH_DETAIL: f32 = 2048.0;      // >= 2048px: level 0
const LOD_THRESHOLD_MEDIUM_HIGH: f32 = 1024.0;     // >= 1024px: level 3
const LOD_THRESHOLD_MEDIUM: f32 = 512.0;           // >= 512px: level 7
const LOD_THRESHOLD_MEDIUM_LOW: f32 = 256.0;      // >= 256px: level 14
const LOD_THRESHOLD_LOW: f32 = 128.0;             // >= 128px: level 21
const LOD_THRESHOLD_VERY_LOW: f32 = 64.0;        // < 64px: level 27

// Task shader: Process each instance and determine LOD
//
// Workgroup size should match Rust side (typically 32 or 64 threads)
@compute @workgroup_size(32)
fn calculate_lod_task(
    @builtin(global_invocation_id) gid: vec3<u32>,
    @builtin(local_invocation_id) lid: vec3<u32>,
) {
    let instance_id = gid.x;

    // Boundary check
    if (instance_id >= arrayLength(&instances)) {
        return;
    }

    let instance = instances[instance_id];

    // ─────────────────────────────────────────────────────────────────────────
    // Step 1: Project instance to screen space
    // ─────────────────────────────────────────────────────────────────────────

    let clip_pos = camera.view_proj * vec4<f32>(instance.position, 1.0);
    let ndc = clip_pos.xy / clip_pos.w;  // Normalized device coordinates
    let screen_pos = (ndc + 1.0) * 0.5 * vec2<f32>(
        f32(camera.viewport_width),
        f32(camera.viewport_height)
    );

    // ─────────────────────────────────────────────────────────────────────────
    // Step 2: Calculate sphere radius from intensity
    // ─────────────────────────────────────────────────────────────────────────

    // Scale intensity to world-space radius
    // intensity [0, 1] → radius [1.0, 3.0]
    let sphere_radius = 1.0 + instance.intensity * 2.0;

    // ─────────────────────────────────────────────────────────────────────────
    // Step 3: Calculate projected screen coverage in pixels
    // ─────────────────────────────────────────────────────────────────────────

    // Distance from camera to instance
    let camera_to_instance = instance.position - camera.camera_pos;
    let distance = length(camera_to_instance);

    // Avoid division by zero
    if (distance < 0.1) {
        // Instance too close to camera, use highest detail
        let payload = LodPayload(0u, 1024u, 341u, instance_id);
        lod_decisions[instance_id] = payload;
        return;
    }

    // Projected radius in pixels (tan(fov/2) ≈ viewport_height / (2 * znear))
    // Simplified: use vertical viewport dimension
    let projected_radius = sphere_radius / distance * f32(camera.viewport_height) / 2.0;
    let screen_coverage = projected_radius * 2.0;  // Diameter in pixels

    // ─────────────────────────────────────────────────────────────────────────
    // Step 4: Select LOD based on screen coverage threshold
    // ─────────────────────────────────────────────────────────────────────────

    var lod_level = 27u;  // Default: lowest detail
    var vertex_count = 64u;
    var triangle_count = 21u;

    if (screen_coverage >= LOD_THRESHOLD_HIGH_DETAIL) {
        lod_level = 0u;
        vertex_count = 1024u;
        triangle_count = 341u;
    } else if (screen_coverage >= LOD_THRESHOLD_MEDIUM_HIGH) {
        lod_level = 3u;
        vertex_count = 896u;
        triangle_count = 298u;
    } else if (screen_coverage >= LOD_THRESHOLD_MEDIUM) {
        lod_level = 7u;
        vertex_count = 512u;
        triangle_count = 170u;
    } else if (screen_coverage >= LOD_THRESHOLD_MEDIUM_LOW) {
        lod_level = 14u;
        vertex_count = 256u;
        triangle_count = 85u;
    } else if (screen_coverage >= LOD_THRESHOLD_LOW) {
        lod_level = 21u;
        vertex_count = 128u;
        triangle_count = 42u;
    } else {
        lod_level = 27u;
        vertex_count = 64u;
        triangle_count = 21u;
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Step 5: Store LOD decision in buffer for render dispatch
    // ─────────────────────────────────────────────────────────────────────────

    let payload = LodPayload(lod_level, vertex_count, triangle_count, instance_id);
    lod_decisions[instance_id] = payload;
}
