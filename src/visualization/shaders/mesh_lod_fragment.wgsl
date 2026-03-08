// src/visualization/shaders/mesh_lod_fragment.wgsl
// Fragment Shader: Heat map shading and intensity-based coloring
//
// Applies per-pixel shading with:
// - Blinn-Phong lighting (diffuse + specular highlights)
// - Heat map tonemap: blue (0.0) → red (0.33) → yellow (0.67) → white (1.0)
// - Intensity-driven color saturation
//
// Output: Final RGBA color

struct FragmentInput {
    @location(0) normal: vec3<f32>,
    @location(1) intensity: f32,
    @location(2) world_pos: vec3<f32>,
}

// ─────────────────────────────────────────────────────────────────────────
// Heat map tonemap function
// ─────────────────────────────────────────────────────────────────────────

fn tonemap_intensity(intensity: f32) -> vec3<f32> {
    // Blue (0.0) → Red (0.33) → Yellow (0.67) → White (1.0)
    // Clamped to [0, 1] range

    let t = clamp(intensity, 0.0, 1.0);

    if (t < 0.33) {
        // Blue → Red transition
        let blend = t * 3.0;  // 0.0 → 1.0 over [0, 0.33]
        return mix(
            vec3<f32>(0.0, 0.0, 1.0),    // Blue
            vec3<f32>(1.0, 0.0, 0.0),    // Red
            blend
        );
    } else if (t < 0.67) {
        // Red → Yellow transition
        let blend = (t - 0.33) * 3.0;  // 0.0 → 1.0 over [0.33, 0.67]
        return mix(
            vec3<f32>(1.0, 0.0, 0.0),    // Red
            vec3<f32>(1.0, 1.0, 0.0),    // Yellow
            blend
        );
    } else {
        // Yellow → White transition
        let blend = (t - 0.67) * 3.0;  // 0.0 → 1.0 over [0.67, 1.0]
        return mix(
            vec3<f32>(1.0, 1.0, 0.0),    // Yellow
            vec3<f32>(1.0, 1.0, 1.0),    // White
            blend
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Blinn-Phong Lighting
// ─────────────────────────────────────────────────────────────────────────

fn blinn_phong_lighting(
    normal: vec3<f32>,
    light_dir: vec3<f32>,
    view_dir: vec3<f32>,
) -> f32 {
    // Diffuse component (Lambertian)
    let diffuse = max(dot(normal, light_dir), 0.2);  // Min ambient = 0.2

    // Specular component (Blinn-Phong)
    let half_dir = normalize(light_dir + view_dir);
    let specular = pow(max(dot(normal, half_dir), 0.0), 32.0);

    return diffuse + specular * 0.3;
}

// ─────────────────────────────────────────────────────────────────────────
// Fragment Shader Entry Point
// ─────────────────────────────────────────────────────────────────────────

@fragment
fn fragment_main(input: FragmentInput) -> @location(0) vec4<f32> {
    // Normalize normal (may have been interpolated)
    let normal = normalize(input.normal);

    // ─────────────────────────────────────────────────────────────────────
    // Lighting calculation
    // ─────────────────────────────────────────────────────────────────────

    // Light direction (fixed world-space light)
    let light_pos = vec3<f32>(10.0, 15.0, 10.0);
    let light_dir = normalize(light_pos - input.world_pos);

    // View direction (from fragment to camera origin)
    let view_dir = normalize(-input.world_pos);

    // Calculate Blinn-Phong lighting
    let light_factor = blinn_phong_lighting(normal, light_dir, view_dir);

    // ─────────────────────────────────────────────────────────────────────
    // Heat map coloring based on intensity
    // ─────────────────────────────────────────────────────────────────────

    let base_color = tonemap_intensity(input.intensity);

    // Apply lighting to base color
    let final_color = base_color * light_factor;

    return vec4<f32>(final_color, 1.0);
}
