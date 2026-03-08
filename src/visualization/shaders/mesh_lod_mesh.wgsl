// src/visualization/shaders/mesh_lod_mesh.wgsl
// Mesh Shader: Adaptive sphere geometry generation per LOD level
//
// Generates sphere vertices procedurally on-the-fly based on LOD level.
// This avoids storing all 28 LOD meshes in GPU memory.
//
// Input: Vertex/instance indices, LOD level
// Output: Vertex positions and normals in world space

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

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) intensity: f32,
    @location(2) world_pos: vec3<f32>,
}

// Bind groups
@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(0) @binding(1) var<storage, read> instances: array<InstanceData>;

// ─────────────────────────────────────────────────────────────────────────
// Sphere geometry generation
// ─────────────────────────────────────────────────────────────────────────

fn sphere_lod_parameters(lod_level: u32) -> vec2<u32> {
    // Returns (latitude_segments, longitude_segments) based on LOD
    // LOD 0: max detail (32×64 segments)
    // LOD 27: min detail (4×8 segments)

    let lat_segments = 4u + (lod_level * 2u) / 2u;      // 4→32
    let lon_segments = 8u + (lod_level * 4u) / 2u;      // 8→64

    return vec2<u32>(lat_segments, lon_segments);
}

fn generate_sphere_vertex(
    lat_idx: u32,
    lon_idx: u32,
    lat_segments: u32,
    lon_segments: u32,
) -> vec3<f32> {
    // Generate sphere vertex at (lat, lon) with normalized radius 1.0

    let pi = 3.141592653589793;
    let tau = 6.283185307179586;

    // Latitude: 0 → π (top to bottom pole)
    let lat = f32(lat_idx) / f32(lat_segments) * pi;

    // Longitude: 0 → 2π (around equator)
    let lon = f32(lon_idx) / f32(lon_segments) * tau;

    // Convert spherical to Cartesian
    let sin_lat = sin(lat);
    let cos_lat = cos(lat);
    let sin_lon = sin(lon);
    let cos_lon = cos(lon);

    let x = sin_lat * cos_lon;
    let y = cos_lat;
    let z = sin_lat * sin_lon;

    return normalize(vec3<f32>(x, y, z));
}

// ─────────────────────────────────────────────────────────────────────────
// Vertex Shader Entry Point
// ─────────────────────────────────────────────────────────────────────────

@vertex
fn vertex_main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
) -> VertexOutput {
    // Determine LOD level from instance index
    // In practice, this would come from the LOD decision buffer
    let lod_level = instance_index % 28u;

    // Get LOD parameters
    let params = sphere_lod_parameters(lod_level);
    let lat_segments = params.x;
    let lon_segments = params.y;

    // Compute latitude and longitude indices from vertex index
    let lat_idx = vertex_index / lon_segments;
    let lon_idx = vertex_index % lon_segments;

    // Skip degenerate vertices
    if (lat_idx >= lat_segments) {
        return VertexOutput(
            vec4<f32>(0.0, 0.0, 0.0, 1.0),
            vec3<f32>(0.0, 1.0, 0.0),
            0.0,
            vec3<f32>(0.0, 0.0, 0.0),
        );
    }

    // Generate normalized sphere vertex
    let normal = generate_sphere_vertex(lat_idx, lon_idx, lat_segments, lon_segments);

    // Scale by instance intensity (sphere radius)
    let instance = instances[instance_index % arrayLength(&instances)];
    let sphere_radius = 1.0 + instance.intensity * 2.0;
    let world_pos = instance.position + normal * sphere_radius;

    // Transform to clip space
    let clip_pos = camera.view_proj * vec4<f32>(world_pos, 1.0);

    return VertexOutput(
        clip_pos,
        normal,           // Surface normal
        instance.intensity, // Pass intensity for heat map coloring
        world_pos,        // World position for lighting
    );
}
