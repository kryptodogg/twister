// src/visualization/shaders/rt_attack.wgsl
// Ray-traced attack visualization shader (Task D.1b)
//
// Generates 3D heat map of attack sources from 128-D TimeGNN embeddings.
// Vulkan ray tracing pipeline: sphere intersection + heat map tonemap.
//
// Performance target: 476fps @ 1920×1080 on RX 6700 XT
// Workgroup: 8×8 (256 threads)

// ─────────────────────────────────────────────────────────────────────────────
// Bind Groups
// ─────────────────────────────────────────────────────────────────────────────

// Output image (storage, write-only)
@group(0) @binding(1) var output_image: texture_storage_2d<rgba8unorm, write>;

// TimeGNN embeddings (128-D per event)
@group(0) @binding(2) var<storage, read> embeddings: array<vec4<f32>>;

// Attack source positions (derived from embeddings)
@group(0) @binding(3) var<storage, read> attack_positions: array<vec4<f32>>;

// Attack intensities (magnitude of embeddings)
@group(0) @binding(4) var<storage, read> attack_intensities: array<f32>;

// Uniforms (camera, viewport)
@group(0) @binding(5) var<uniform> params: RtParams;

// ─────────────────────────────────────────────────────────────────────────────
// Structures
// ─────────────────────────────────────────────────────────────────────────────

struct RtParams {
    // Camera position (vec3 + padding)
    camera_pos: vec4<f32>,
    // Camera forward direction (vec3 + padding)
    camera_forward: vec4<f32>,
    // Camera right direction (vec3 + padding)
    camera_right: vec4<f32>,
    // Camera up direction (vec3 + padding)
    camera_up: vec4<f32>,
    // Viewport dimensions
    viewport_width: u32,
    viewport_height: u32,
    // Number of attack sources
    num_attacks: u32,
    // Max bounces (reserved)
    max_bounces: u32,
}

struct RayPayload {
    hit_color: vec3<f32>,
    hit_distance: f32,
    hit_intensity: f32,
}

// ─────────────────────────────────────────────────────────────────────────────
// Heat map tonemap function
// ─────────────────────────────────────────────────────────────────────────────

/// Convert intensity [0,1] to heat map color (blue→red→yellow→white)
fn tonemap_attack_intensity(intensity: f32) -> vec3<f32> {
    let i = clamp(intensity, 0.0, 1.0);

    if i < 0.33 {
        // Blue (0,0,1) → Red (1,0,0)
        let t = i * 3.0;  // [0, 1] over [0, 0.33]
        return mix(vec3<f32>(0.0, 0.0, 1.0), vec3<f32>(1.0, 0.0, 0.0), t);
    } else if i < 0.67 {
        // Red (1,0,0) → Yellow (1,1,0)
        let t = (i - 0.33) * 3.0;  // [0, 1] over [0.33, 0.67]
        return mix(vec3<f32>(1.0, 0.0, 0.0), vec3<f32>(1.0, 1.0, 0.0), t);
    } else {
        // Yellow (1,1,0) → White (1,1,1)
        let t = (i - 0.67) * 3.0;  // [0, 1] over [0.67, 1.0]
        return mix(vec3<f32>(1.0, 1.0, 0.0), vec3<f32>(1.0, 1.0, 1.0), t);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Ray-sphere intersection
// ─────────────────────────────────────────────────────────────────────────────

/// Ray-sphere intersection (attack source as sphere)
/// Returns distance to closest intersection (or -1 if no hit)
fn ray_sphere_intersect(
    ray_origin: vec3<f32>,
    ray_dir: vec3<f32>,
    sphere_center: vec3<f32>,
    sphere_radius: f32,
) -> f32 {
    let oc = ray_origin - sphere_center;
    let a = dot(ray_dir, ray_dir);
    let b = 2.0 * dot(oc, ray_dir);
    let c = dot(oc, oc) - sphere_radius * sphere_radius;

    let discriminant = b * b - 4.0 * a * c;

    if discriminant < 0.0 {
        return -1.0;  // No intersection
    }

    let t = (-b - sqrt(discriminant)) / (2.0 * a);
    if t > 0.01 {
        return t;
    }

    return -1.0;
}

// ─────────────────────────────────────────────────────────────────────────────
// Attack field sampling
// ─────────────────────────────────────────────────────────────────────────────

/// Sample attack field: test ray against all attack sources
fn sample_attack_field(ray_origin: vec3<f32>, ray_dir: vec3<f32>) -> RayPayload {
    var result = RayPayload(vec3<f32>(0.0), 1e6, 0.0);

    // Test ray against each attack source
    for (var i = 0u; i < min(params.num_attacks, 32u); i++) {
        let attack_pos = attack_positions[i].xyz;
        let attack_int = attack_intensities[i];

        // Sphere radius proportional to intensity
        let sphere_radius = sqrt(attack_int) * 10.0;  // Scale for visibility

        let t = ray_sphere_intersect(ray_origin, ray_dir, attack_pos, sphere_radius);

        if t > 0.0 && t < result.hit_distance {
            result.hit_distance = t;
            result.hit_intensity = attack_int;

            // Color based on distance for depth cue
            let color_far = tonemap_attack_intensity(attack_int);
            let color_near = vec3<f32>(1.0);  // White at close range
            result.hit_color = mix(color_near, color_far, min(t / 100.0, 1.0));
        }
    }

    return result;
}

// ─────────────────────────────────────────────────────────────────────────────
// Main compute shader
// ─────────────────────────────────────────────────────────────────────────────

/// Main ray tracing compute shader
/// Dispatched in 8×8 workgroups (256 threads per group)
@compute @workgroup_size(8, 8, 1)
fn trace_attack_rays(@builtin(global_invocation_id) gid: vec3<u32>) {
    let pixel_x = gid.x;
    let pixel_y = gid.y;

    // Bounds check
    if pixel_x >= params.viewport_width || pixel_y >= params.viewport_height {
        return;
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Generate primary ray
    // ─────────────────────────────────────────────────────────────────────────

    // Normalized pixel coordinates [0, 1]
    let uv = vec2<f32>(f32(pixel_x), f32(pixel_y)) / vec2<f32>(
        f32(params.viewport_width),
        f32(params.viewport_height),
    );

    // Convert to NDC [-1, 1]
    let ray_ndc = uv * 2.0 - 1.0;

    // Aspect ratio correction
    let aspect = f32(params.viewport_width) / f32(params.viewport_height);

    // Generate ray direction (perspective projection)
    let ray_direction = normalize(
        params.camera_forward.xyz
            + ray_ndc.x * params.camera_right.xyz * aspect
            + ray_ndc.y * params.camera_up.xyz,
    );

    let ray_origin = params.camera_pos.xyz;

    // ─────────────────────────────────────────────────────────────────────────
    // Trace ray through attack field
    // ─────────────────────────────────────────────────────────────────────────

    var payload = sample_attack_field(ray_origin, ray_direction);

    // If no hit found, use background color
    var final_color = vec3<f32>(0.0);

    if payload.hit_distance < 1e5 {
        // Apply heat map based on intensity
        final_color = tonemap_attack_intensity(payload.hit_intensity);

        // Optional: apply distance fog for depth perception
        let fog_factor = exp(-payload.hit_distance / 200.0);
        final_color = mix(vec3<f32>(0.0), final_color, fog_factor);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Write output
    // ─────────────────────────────────────────────────────────────────────────

    let pixel_coord = vec2<i32>(i32(pixel_x), i32(pixel_y));
    let output_color = vec4<f32>(final_color, 1.0);

    textureStore(output_image, pixel_coord, output_color);
}
