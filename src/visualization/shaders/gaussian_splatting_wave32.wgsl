// shaders/gaussian_splatting_wave32.wgsl
//
// GPU Gaussian Splatting compute shader - Wave32 Optimized
// Uses @workgroup_size(32, 1, 1) to hint Wave32 execution (32 threads = 1 Wave32)
// This tests whether native ALU width (32) provides better performance than Wave64

@group(0) @binding(0) var<storage, read_write> p_azimuth: array<f32>;
@group(0) @binding(1) var<storage, read_write> p_elevation: array<f32>;
@group(0) @binding(2) var<storage, read_write> p_frequency: array<f32>;
@group(0) @binding(3) var<storage, read_write> p_grid_hash: array<u32>;
@group(0) @binding(4) var<storage, read_write> p_sorted_idx: array<u32>;

/// Compute gaussian splatting kernel - Wave32 variant
/// Workgroup size 32x1 = 32 threads = exactly 1 Wave32 execution unit
@compute @workgroup_size(32, 1, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    // Map linear thread ID to 2D pixel coordinates
    // With 32x1 threads, we need to iterate over output pixels differently
    let threads_per_row = 32u;
    let pixel_row = global_id.y;
    let pixel_col_base = global_id.x;

    // Process multiple pixels per thread to cover the 1024x1024 output
    let pixels_per_thread = (1024u * 1024u) / (32u * 1u) / 1024u; // Approx 32 pixels per thread

    for (var p: u32 = 0u; p < pixels_per_thread; p = p + 1u) {
        let pixel_idx = pixel_row * 1024u + (pixel_col_base * pixels_per_thread + p);

        if pixel_idx >= 1024u * 1024u {
            return;
        }

        let grid_x = pixel_idx % 1024u;
        let grid_y = pixel_idx / 1024u;

        // Accumulator for final pixel color (RGBA)
        var accumulated_color: vec4<f32> = vec4<f32>(0.0, 0.0, 0.0, 0.0);

        // Iterate over all particles
        for (var i: u32 = 0u; i < 10000u; i = i + 1u) {
            // Load particle attributes from SoA buffers
            let azimuth = p_azimuth[i];
            let elevation = p_elevation[i];
            let frequency = p_frequency[i];

            // Simple gaussian kernel evaluation
            let az_normalized = (azimuth / 6.28318) * 1024.0;
            let el_normalized = ((elevation + 1.57079) / 3.14159) * 1024.0;

            let dx = f32(grid_x) - az_normalized;
            let dy = f32(grid_y) - el_normalized;
            let dist_sq = dx * dx + dy * dy;

            // Gaussian kernel: exp(-dist^2 / (2*sigma^2))
            let sigma = 2.0;
            let sigma_sq = sigma * sigma;
            let gaussian = exp(-dist_sq / (2.0 * sigma_sq));

            // Frequency as intensity scale (log space)
            let freq_normalized = frequency / 12288000.0;
            let intensity = gaussian * freq_normalized;

            // Tonemap: frequency -> color (blue->red->yellow->white)
            var color: vec3<f32>;
            if freq_normalized < 0.33 {
                color = vec3<f32>(0.0, 0.0, 1.0) + (freq_normalized / 0.33) * vec3<f32>(1.0, 0.0, -1.0);
            } else if freq_normalized < 0.67 {
                color = vec3<f32>(1.0, 0.0, 0.0) + ((freq_normalized - 0.33) / 0.34) * vec3<f32>(0.0, 1.0, 0.0);
            } else {
                color = vec3<f32>(1.0, 1.0, 0.0) + ((freq_normalized - 0.67) / 0.33) * vec3<f32>(0.0, 0.0, 1.0);
            }

            // Accumulate with blending
            accumulated_color = accumulated_color + vec4<f32>(color * intensity, intensity);
        }

        // Normalize by accumulated alpha
        if accumulated_color.w > 0.0001 {
            accumulated_color.x = accumulated_color.x / accumulated_color.w;
            accumulated_color.y = accumulated_color.y / accumulated_color.w;
            accumulated_color.z = accumulated_color.z / accumulated_color.w;
        }
        accumulated_color.w = min(accumulated_color.w, 1.0);

        // Write to output buffer
        p_grid_hash[pixel_idx] = bitcast<u32>(accumulated_color.w);
    }
}
