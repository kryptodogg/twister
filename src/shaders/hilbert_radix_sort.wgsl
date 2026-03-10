// src/shaders/hilbert_radix_sort.wgsl
// GPU-Accelerated Radix Sort for Hilbert-Indexed Particles
//
// Sorts Hilbert keys using parallel radix sort (8-bit passes)
// Enables GPU-side sorting without CPU roundtrip

struct HilbertKey {
    index: u64,
    particle_idx: u32,
}

// Input: Unsorted Hilbert keys
@group(0) @binding(0)
var<storage, read_write> keys_in: array<HilbertKey>;

// Output: Sorted keys
@group(0) @binding(1)
var<storage, read_write> keys_out: array<HilbertKey>;

// Temporary buffers for radix sort passes
@group(0) @binding(2)
var<storage, read_write> temp_buffer: array<HilbertKey>;

// Uniforms
@group(0) @binding(3)
var<uniform> params: SortParams;

struct SortParams {
    element_count: u32,
    radix_shift: u32,
    radix_mask: u32,
    pass_number: u32,
}

// Histogram for radix sort
@group(0) @binding(4)
var<storage, read_write> histogram: array<atomic<u32>, 256>;

// Scan result for parallel prefix sum
@group(0) @binding(5)
var<storage, read_write> scan_result: array<u32>;

// Kernel 1: Count occurrences of each radix digit
@compute
@workgroup_size(256, 1, 1)
fn histogram_kernel(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if idx >= params.element_count {
        return;
    }

    let key = keys_in[idx];
    let digit = (key.index >> params.radix_shift) & u64(params.radix_mask);
    atomicAdd(&histogram[digit], 1u);
}

// Kernel 2: Exclusive prefix scan (barriers separate passes)
@compute
@workgroup_size(256, 1, 1)
fn scan_kernel(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;

    // Simple exclusive scan (non-optimized version)
    // Production: use Blelloch scan for O(log n) complexity
    var sum = 0u;
    for (var i = 0u; i < idx; i++) {
        sum += atomicLoad(&histogram[i]);
    }
    scan_result[idx] = sum;
}

// Kernel 3: Reorder keys based on radix digit and scan result
@compute
@workgroup_size(256, 1, 1)
fn scatter_kernel(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if idx >= params.element_count {
        return;
    }

    let key = keys_in[idx];
    let digit = (key.index >> params.radix_shift) & u64(params.radix_mask);
    let position = scan_result[digit];

    // Atomic increment to get unique position for this digit
    let out_idx = atomicAdd(&scan_result[digit + 256u], 1u);
    temp_buffer[position + out_idx] = key;
}

// Kernel 4: Copy sorted result back
@compute
@workgroup_size(256, 1, 1)
fn copy_kernel(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if idx >= params.element_count {
        return;
    }

    keys_out[idx] = temp_buffer[idx];
}
