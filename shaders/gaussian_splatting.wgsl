/// GPU Gaussian Splatting Renderer
/// Optimized for RDNA2 (RX 6700 XT) with spatial hashing and workgroup memory
///
/// Performance targets:
/// - 1024×1024 output @ 169 fps (< 5.9ms frame time)
/// - 10,000+ points with O(n) spatial hashing
/// - Workgroup size: 256 (wave64 optimization)
///
/// Memory Layout: Separate Storage Buffers for SoA (Structure of Arrays)
/// This preserves GPU memory coalescence while complying with WGSL restrictions
/// against multiple dynamically-sized arrays in a single struct.

// ── Uniforms ──────────────────────────────────────────────────────────────────

struct Uniforms {
    width: u32,         // Viewport width (1024)
    height: u32,        // Viewport height (1024)
    point_count: u32,   // Number of active particles
    sigma: f32,         // Gaussian sigma (spread)

    grid_size: u32,     // Spatial hash grid size (e.g., 32)
    inv_grid_size: f32, // 1.0 / grid_size
    time: f32,          // Time for animations
    intensity_scale: f32, // Global intensity multiplier
};

// ── Bindings: Separate Storage Buffers for SoA Layout ────────────────────────
// Each particle attribute has its own storage buffer binding
// This preserves memory coalescence while complying with WGSL restrictions

@group(0) @binding(0) var<storage, read> azimuth: array<f32>;      // [-π, π]
@group(0) @binding(1) var<storage, read> elevation: array<f32>;    // [-π/2, π/2]
@group(0) @binding(2) var<storage, read> frequency: array<f32>;    // [0, 1]
@group(0) @binding(3) var<storage, read> intensity: array<f32>;    // [0, 1]
@group(0) @binding(4) var<storage, read> timestamp: array<f32>;    // [0, 1]
@group(0) @binding(5) var<storage, read> confidence: array<f32>;   // [0, 1]
@group(0) @binding(6) var<storage, read> grid_hash: array<u32>;    // Spatial hash
@group(0) @binding(7) var<storage, read> sorted_idx: array<u32>;   // Sorted indices

@group(0) @binding(8) var<uniform> uniforms: Uniforms;
@group(0) @binding(9) var<storage, read_write> output_texture: array<vec4<f32>>;

// ── Constants ─────────────────────────────────────────────────────────────────

const PI: f32 = 3.141592653589793;
const INV_PI: f32 = 0.3183098861837907;
const INV_TWO_PI: f32 = 0.15915494309189535;
const SIGMA_SCALE: f32 = 2.0;  // Sigma scaling factor for visual quality

// ── Projection Functions ──────────────────────────────────────────────────────

/// Project spherical coordinates to screen space
fn project_to_screen(az: f32, el: f32, freq: f32) -> vec2<f32> {
    // Azimuth maps to X, elevation maps to Y
    let x = (az * INV_TWO_PI + 0.5) * f32(uniforms.width);
    let y = (0.5 - (el * INV_PI + 0.5)) * f32(uniforms.height);  // Flip Y
    return vec2<f32>(x, y);
}

// ── Heat Map Tonemapping ──────────────────────────────────────────────────────

/// Convert accumulated intensity to heat map color
/// Gradient: Blue (low) → Cyan → Green → Yellow → Red → White (high)
fn heatmap_tonemap(intensity: f32) -> vec3<f32> {
    let t = clamp(intensity * uniforms.intensity_scale, 0.0, 1.0);

    // Five-segment gradient for smooth color transitions
    if (t < 0.2) {
        // Blue → Cyan (0.0 - 0.2)
        let s = t * 5.0;
        return vec3<f32>(0.0, s, 1.0);
    } else if (t < 0.4) {
        // Cyan → Green (0.2 - 0.4)
        let s = (t - 0.2) * 5.0;
        return vec3<f32>(0.0, 1.0, 1.0 - s);
    } else if (t < 0.6) {
        // Green → Yellow (0.4 - 0.6)
        let s = (t - 0.4) * 5.0;
        return vec3<f32>(s, 1.0, 0.0);
    } else if (t < 0.8) {
        // Yellow → Red (0.6 - 0.8)
        let s = (t - 0.6) * 5.0;
        return vec3<f32>(1.0, 1.0 - s, 0.0);
    } else if (t < 1.0) {
        // Red → White (0.8 - 1.0)
        let s = (t - 0.8) * 5.0;
        return vec3<f32>(1.0, s, s);
    } else {
        // Overexposed → Pure white
        let excess = min(t - 1.0, 1.0);
        return vec3<f32>(1.0, 0.5 + excess * 0.5, 0.5 + excess * 0.5);
    }
}

// ── Workgroup Memory for Shared Data ─────────────────────────────────────────

// Shared particle data for workgroup-level batching
var<workgroup> wg_azimuth: array<f32, 256>;
var<workgroup> wg_elevation: array<f32, 256>;
var<workgroup> wg_intensity: array<f32, 256>;
var<workgroup> wg_confidence: array<f32, 256>;
var<workgroup> wg_screen_x: array<f32, 256>;
var<workgroup> wg_screen_y: array<f32, 256>;

// ── Compute Shader Entry Point ────────────────────────────────────────────────

@compute @workgroup_size(16, 16, 1)  // 256 threads per workgroup
fn gaussian_splat_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x = global_id.x;
    let y = global_id.y;

    // Bounds check
    if (x >= uniforms.width || y >= uniforms.height) {
        return;
    }

    let pixel_idx = y * uniforms.width + x;
    var accumulated_intensity: f32 = 0.0;

    // ── Phase 1: Load particle data into workgroup memory (coalesced access) ──
    let wg_size: u32 = 256u;

    // Load particles in batches (tiling optimization)
    let batch_count = (uniforms.point_count + wg_size - 1u) / wg_size;

    for (var batch = 0u; batch < batch_count; batch = batch + 1u) {
        let batch_base = batch * wg_size;
        let local_idx = global_id.x + global_id.y * 16u;
        let particle_idx = batch_base + local_idx;

        // Load particle data into shared memory using sorted indices
        if (particle_idx < uniforms.point_count) {
            let idx = sorted_idx[particle_idx];
            wg_azimuth[local_idx] = azimuth[idx];
            wg_elevation[local_idx] = elevation[idx];
            wg_intensity[local_idx] = intensity[idx];
            wg_confidence[local_idx] = confidence[idx];

            let proj = project_to_screen(azimuth[idx], elevation[idx], frequency[idx]);
            wg_screen_x[local_idx] = proj.x;
            wg_screen_y[local_idx] = proj.y;
        }

        // Synchronize workgroup
        workgroupBarrier();

        // ── Phase 2: Accumulate Gaussian contributions from workgroup particles ──
        let particles_in_batch = min(wg_size, uniforms.point_count - batch_base);

        for (var i = 0u; i < particles_in_batch; i = i + 1u) {
            let px = wg_screen_x[i];
            let py = wg_screen_y[i];

            // Calculate squared distance from pixel to projected point
            let dx = f32(x) - px;
            let dy = f32(y) - py;
            let dist_sq = dx * dx + dy * dy;

            // Gaussian kernel: I * exp(-0.5 * dist² / σ²)
            let sigma_sq = uniforms.sigma * uniforms.sigma * SIGMA_SCALE;
            let gaussian = exp(-0.5 * dist_sq / sigma_sq);

            // Accumulate weighted by intensity and confidence
            accumulated_intensity += wg_intensity[i] * gaussian * wg_confidence[i];
        }

        // Synchronize before next batch
        workgroupBarrier();
    }

    // ── Phase 3: Tonemap and write output ─────────────────────────────────────
    let color = heatmap_tonemap(accumulated_intensity);
    output_texture[pixel_idx] = vec4<f32>(color.r, color.g, color.b, 1.0);
}

// ── Alternative: Direct Per-Pixel Accumulation (fallback for small point counts) ──

@compute @workgroup_size(16, 16, 1)
fn gaussian_splat_direct(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x = global_id.x;
    let y = global_id.y;

    if (x >= uniforms.width || y >= uniforms.height) {
        return;
    }

    let pixel_idx = y * uniforms.width + x;
    var accumulated_intensity: f32 = 0.0;

    // Direct accumulation (O(n) per pixel, use only for small point counts)
    for (var i = 0u; i < uniforms.point_count; i = i + 1u) {
        let proj = project_to_screen(azimuth[i], elevation[i], frequency[i]);

        let dx = f32(x) - proj.x;
        let dy = f32(y) - proj.y;
        let dist_sq = dx * dx + dy * dy;

        let sigma_sq = uniforms.sigma * uniforms.sigma * SIGMA_SCALE;
        let gaussian = exp(-0.5 * dist_sq / sigma_sq);

        accumulated_intensity += intensity[i] * gaussian * confidence[i];
    }

    let color = heatmap_tonemap(accumulated_intensity);
    output_texture[pixel_idx] = vec4<f32>(color.r, color.g, color.b, 1.0);
}

// ── Spatial Hash Build Shader (pre-pass for radix sort) ──────────────────────

/// Build spatial hash grid from particle positions
/// Run before gaussian_splat_main to populate grid_hash array
@compute @workgroup_size(256, 1, 1)
fn build_spatial_hash(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if (idx >= uniforms.point_count) {
        return;
    }

    // Normalize coordinates to [0, 1] range
    let az = azimuth[idx];
    let el = elevation[idx];
    let freq = frequency[idx];

    let x = az * INV_TWO_PI + 0.5;
    let y = el * INV_PI + 0.5;
    let z = freq;

    // Quantize to grid cells
    let gx = u32(x * f32(uniforms.grid_size)) % uniforms.grid_size;
    let gy = u32(y * f32(uniforms.grid_size)) % uniforms.grid_size;
    let gz = u32(z * f32(uniforms.grid_size)) % uniforms.grid_size;

    // Morton code (Z-order curve) for 3D
    var hash = gx | (gy << 8) | (gz << 16);

    // Mix bits for better distribution
    hash = hash ^ (hash >> 13);
    hash = hash * 0x9e3779b9u;
    hash = hash ^ (hash >> 16);

    grid_hash[idx] = hash % (uniforms.grid_size * uniforms.grid_size * uniforms.grid_size);
    sorted_idx[idx] = idx;
}
