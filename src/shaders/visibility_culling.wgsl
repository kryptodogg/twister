// src/shaders/visibility_culling.wgsl
// GPU Hi-Z Frustum Culling + Occlusion Testing + Auto-LOD
//
// Per-particle compute: Test frustum + occlusion + screen-space clustering
// Output: Indirect draw buffer with only visible particle IDs

struct FrustumPlane {
    normal: vec3<f32>,
    distance: f32,
}

struct CameraFrustum {
    planes: array<FrustumPlane, 6>,
    camera_pos: vec3<f32>,
    _padding: f32,
}

struct FieldParticle {
    position: vec3<f32>,
    phase_amp: vec2<f32>,
    material: vec3<f32>,
    energy_gradient: f32,
    _padding: f32,
}

struct IndirectDrawCommand {
    vertex_count: u32,
    instance_count: atomic<u32>,
    first_vertex: u32,
    first_instance: u32,
}

struct CullingStats {
    total_particles: u32,
    frustum_culled: atomic<u32>,
    occlusion_culled: atomic<u32>,
    lod_merged: atomic<u32>,
    visible_particles: atomic<u32>,
}

// ─────────────────────────────────────────────────────────────────────
// INPUT BINDINGS
// ─────────────────────────────────────────────────────────────────────

// Sorted particles from Mamba forward pass
@group(0) @binding(0)
var<storage, read> particles: array<FieldParticle>;

// Camera frustum planes
@group(0) @binding(1)
var<uniform> frustum: CameraFrustum;

// View-Projection matrix for screen-space projection
@group(0) @binding(2)
var<uniform> view_proj: mat4x4<f32>;

// Hierarchical Z-Buffer (depth pyramid from previous frame)
@group(0) @binding(3)
var hiz_pyramid: texture_2d<f32>;

@group(0) @binding(4)
var hiz_sampler: sampler;

// ─────────────────────────────────────────────────────────────────────
// OUTPUT BINDINGS
// ─────────────────────────────────────────────────────────────────────

// Visible particle IDs (compacted output)
@group(0) @binding(5)
var<storage, read_write> visible_particle_ids: array<u32>;

// Indirect draw command (counter atomically incremented)
@group(0) @binding(6)
var<storage, read_write> indirect_command: IndirectDrawCommand;

// Statistics
@group(0) @binding(7)
var<storage, read_write> stats: CullingStats;

// ─────────────────────────────────────────────────────────────────────
// HELPER FUNCTIONS
// ─────────────────────────────────────────────────────────────────────

/// Test if point is inside frustum (all 6 plane checks)
fn frustum_contains(pos: vec3<f32>) -> bool {
    for (var i = 0u; i < 6u; i++) {
        let plane = frustum.planes[i];
        let dist = dot(plane.normal, pos) + plane.distance;
        if (dist < 0.0) {
            return false;
        }
    }
    return true;
}

/// Project position to screen space
fn project_to_screen(pos: vec3<f32>) -> vec2<f32> {
    let clip_pos = view_proj * vec4<f32>(pos, 1.0);
    let ndc = clip_pos.xy / clip_pos.w;
    // NDC [-1, 1] → Screen [0, 1]
    return ndc * 0.5 + vec2<f32>(0.5);
}

/// Read depth from Hi-Z pyramid at screen position
/// Samples the appropriate mip level based on LOD
fn sample_hiz_depth(screen_pos: vec2<f32>, mip_level: u32) -> f32 {
    let uv = screen_pos;
    // Sample with manual mip selection
    let texel = textureSampleLevel(hiz_pyramid, hiz_sampler, uv, f32(mip_level));
    return texel.r;
}

/// Occlusion test: is this particle occluded by Hi-Z?
fn is_occluded(pos: vec3<f32>, projected_depth: f32) -> bool {
    let screen_pos = project_to_screen(pos);

    // Sample Hi-Z at appropriate LOD (coarser = fewer false positives)
    let hiz_depth = sample_hiz_depth(screen_pos, 2u);  // Mip level 2 = 1/4 resolution

    // If particle depth > Hi-Z depth, it's behind something → occluded
    return projected_depth > hiz_depth;
}

/// Auto-LOD: Estimate screen-space size of particle cluster
/// Returns true if cluster projects to <= 1 pixel
fn should_merge_for_lod(pos: vec3<f32>, screen_size: f32) -> bool {
    return screen_size < 1.0;
}

/// Distance-based LOD: coarse particles at distance
fn estimate_screen_size(world_pos: vec3<f32>, particle_radius: f32) -> f32 {
    let to_camera = normalize(frustum.camera_pos - world_pos);
    let dist = length(frustum.camera_pos - world_pos);

    // Project particle radius to screen
    let clip_pos = view_proj * vec4<f32>(world_pos, 1.0);
    let screen_proj = clip_pos.xy / clip_pos.w;

    // Approximate: radius / distance (in normalized device coordinates)
    return particle_radius / max(dist, 0.1);
}

// ─────────────────────────────────────────────────────────────────────
// MAIN CULLING KERNEL
// ─────────────────────────────────────────────────────────────────────

@compute
@workgroup_size(256, 1, 1)
fn cull_particles(@builtin(global_invocation_id) gid: vec3<u32>) {
    let particle_idx = gid.x;

    // Early exit if beyond particle buffer
    if (particle_idx >= arrayLength(&particles)) {
        return;
    }

    atomicAdd(&stats.total_particles, 1u);

    let particle = particles[particle_idx];
    let pos = particle.position;

    // ─ STAGE 1: Frustum Culling ─
    if (!frustum_contains(pos)) {
        atomicAdd(&stats.frustum_culled, 1u);
        return;
    }

    // ─ STAGE 2: Project to Screen ─
    let clip_pos = view_proj * vec4<f32>(pos, 1.0);
    let projected_depth = clip_pos.z / clip_pos.w;

    // ─ STAGE 3: Occlusion Testing (Hi-Z) ─
    if (is_occluded(pos, projected_depth)) {
        atomicAdd(&stats.occlusion_culled, 1u);
        return;
    }

    // ─ STAGE 4: Auto-LOD (Cluster Merging) ─
    let screen_size = estimate_screen_size(pos, 0.1);  // 0.1m particle radius
    if (should_merge_for_lod(pos, screen_size)) {
        // Would merge this into parent cluster (deferred to compaction pass)
        atomicAdd(&stats.lod_merged, 1u);
        // For now: still include (compaction optimization deferred)
    }

    // ─ STAGE 5: PASS - Add to visible list ─
    let visible_idx = atomicAdd(&indirect_command.instance_count, 1u);
    if (visible_idx < arrayLength(&visible_particle_ids)) {
        visible_particle_ids[visible_idx] = particle_idx;
    }
    atomicAdd(&stats.visible_particles, 1u);
}

// ─────────────────────────────────────────────────────────────────────
// OPTIONAL: Indirect Command Finalization
// (Run after culling to prepare draw call)
// ─────────────────────────────────────────────────────────────────────

@compute
@workgroup_size(1, 1, 1)
fn finalize_indirect_command() {
    // Set up indirect draw parameters based on visible count
    indirect_command.vertex_count = 6u;      // 2 triangles per quad
    indirect_command.first_vertex = 0u;
    indirect_command.first_instance = 0u;
    // instance_count already set by atomic increments above
}
