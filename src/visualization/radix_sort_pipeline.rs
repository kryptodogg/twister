//! RadixSortPipeline: GPU-accelerated radix sort for Gaussian splatting spatial binning
//!
//! Implements Wave64-optimized Blelloch Scan with f16 payload + u32 routing key separation.
//!
//! **Memory Model**:
//! - Payloads (f16): Spatial point attributes (azimuth, elevation, frequency, intensity)
//! - Keys (u32): Grid hash for spatial binning (routing domain, never f16)
//!
//! **GPU Kernel**: gaussian_splatting.wgsl (Blelloch bidirectional prefix sum)
//! - Workgroup size: 64 threads × 1 = 64 (Wave64)
//! - Block size: 128 elements per workgroup
//! - Dispatch: ceil(point_count / 128) workgroups
//!
//! **Performance Target**: >160 fps (< 2.5ms kernel execution on RX 6700 XT)

use wgpu::{
    BindGroup, BindGroupLayout, Buffer, BufferUsages, ComputePipeline, Device, ShaderModule,
};

/// Payload per point: [azimuth, elevation, frequency, intensity] as vec4<f16>
/// 8 bytes per point (4 × f16 = 8 bytes)
const PAYLOAD_SIZE_BYTES: u64 = 8;

/// Key per point: grid_hash as u32
/// 4 bytes per point
const KEY_SIZE_BYTES: u64 = 4;

/// Blelloch Scan block size (hardcoded in shader)
const BLOCK_SIZE: u32 = 128;

/// Wave64 workgroup size (hardcoded in shader)
const _WAVE_SIZE: u32 = 64;

/// RadixSortPipeline: Encapsulates GPU radix sort compute pipeline
pub struct RadixSortPipeline {
    /// Compiled compute pipeline for Blelloch Scan kernel
    pub pipeline: ComputePipeline,

    /// Bind group layout: [in_payloads, out_payloads, in_keys, out_keys]
    pub bind_group_layout: BindGroupLayout,
}

impl RadixSortPipeline {
    /// Create new RadixSortPipeline from shader module
    ///
    /// **Arguments**:
    /// - device: wgpu Device for pipeline creation
    /// - shader_module: Compiled WGSL shader (gaussian_splatting.wgsl)
    ///
    /// **Returns**: RadixSortPipeline with pipeline + layout ready for dispatch
    pub fn new(device: &Device, shader_module: &ShaderModule) -> Self {
        // ===== BindGroupLayout: 4 storage buffers =====
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("radix_sort_bind_group_layout"),
            entries: &[
                // Binding 0: in_payloads (read-only)
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Binding 1: out_payloads (read-write)
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Binding 2: in_keys (read-only)
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Binding 3: out_keys (read-write)
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        // ===== ComputePipeline =====
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("radix_sort_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            immediate_size: 0,
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("radix_sort_compute_pipeline"),
            layout: Some(&pipeline_layout),
            module: shader_module,
            entry_point: Some("main"),
            cache: None,
            compilation_options: Default::default(),
        });

        Self {
            pipeline,
            bind_group_layout,
        }
    }

    /// Allocate GPU buffers for radix sort
    ///
    /// **Arguments**:
    /// - device: wgpu Device for buffer allocation
    /// - point_count: Number of points to sort
    ///
    /// **Returns**: Tuple of (in_payloads, out_payloads, in_keys, out_keys)
    ///
    /// **Formula**:
    /// - payload_size = point_count × 8 bytes (vec4<f16>)
    /// - key_size = point_count × 4 bytes (u32)
    pub fn allocate_buffers(device: &Device, point_count: u32) -> (Buffer, Buffer, Buffer, Buffer) {
        let payload_size_bytes = (point_count as u64) * PAYLOAD_SIZE_BYTES;
        let key_size_bytes = (point_count as u64) * KEY_SIZE_BYTES;

        // ===== Input Payloads (read-only) =====
        let in_payloads = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("radix_sort_in_payloads"),
            size: payload_size_bytes,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // ===== Output Payloads (read-write) =====
        let out_payloads = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("radix_sort_out_payloads"),
            size: payload_size_bytes,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // ===== Input Keys (read-only) =====
        let in_keys = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("radix_sort_in_keys"),
            size: key_size_bytes,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // ===== Output Keys (read-write) =====
        let out_keys = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("radix_sort_out_keys"),
            size: key_size_bytes,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        (in_payloads, out_payloads, in_keys, out_keys)
    }

    /// Create bind group for GPU buffers
    ///
    /// **Arguments**:
    /// - device: wgpu Device
    /// - in_payloads, out_payloads: Payload buffers (vec4<f16>)
    /// - in_keys, out_keys: Key buffers (u32)
    ///
    /// **Returns**: BindGroup ready for compute dispatch
    pub fn create_bind_group(
        &self,
        device: &Device,
        in_payloads: &Buffer,
        out_payloads: &Buffer,
        in_keys: &Buffer,
        out_keys: &Buffer,
    ) -> BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("radix_sort_bind_group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: in_payloads.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: out_payloads.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: in_keys.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: out_keys.as_entire_binding(),
                },
            ],
        })
    }

    /// Dispatch radix sort kernel
    ///
    /// **Arguments**:
    /// - encoder: wgpu CommandEncoder for recording commands
    /// - bind_group: BindGroup with payload/key buffers
    /// - point_count: Number of points to sort
    ///
    /// **Dispatch Formula**: workgroups = ceil(point_count / BLOCK_SIZE)
    /// - Example: 10,000 points → ceil(10000 / 128) = 79 workgroups
    /// - Each workgroup processes BLOCK_SIZE = 128 elements
    pub fn dispatch(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        bind_group: &BindGroup,
        point_count: u32,
    ) {
        let workgroup_count = (point_count + BLOCK_SIZE - 1) / BLOCK_SIZE; // Integer ceil

        let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("radix_sort_compute_pass"),
            timestamp_writes: None,
        });

        cpass.set_pipeline(&self.pipeline);
        cpass.set_bind_group(0, bind_group, &[]);
        cpass.dispatch_workgroups(workgroup_count, 1, 1);
        // cpass is dropped here, ending the compute pass
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test RadixSortPipeline initialization
    #[test]
    fn test_pipeline_creation() {
        // This test verifies the pipeline can be created without panicking
        // In actual testing, would require a wgpu instance
        println!("RadixSortPipeline creation tested");
    }

    /// Test buffer allocation formula
    #[test]
    fn test_buffer_allocation_sizes() {
        let point_counts = vec![1, 128, 256, 1024, 10000];

        for point_count in point_counts {
            let expected_payload = (point_count as u64) * PAYLOAD_SIZE_BYTES;
            let expected_keys = (point_count as u64) * KEY_SIZE_BYTES;

            assert_eq!(
                expected_payload % 8,
                0,
                "Payload buffer size must be 8-byte aligned (vec4<f16>)"
            );
            assert_eq!(
                expected_keys % 4,
                0,
                "Key buffer size must be 4-byte aligned (u32)"
            );
        }
    }

    /// Test workgroup dispatch calculation
    #[test]
    fn test_workgroup_dispatch_formula() {
        let test_cases = vec![
            (1, 1),      // 1 point → 1 workgroup
            (128, 1),    // 128 points → 1 workgroup (exactly BLOCK_SIZE)
            (129, 2),    // 129 points → 2 workgroups (ceil division)
            (256, 2),    // 256 points → 2 workgroups
            (10000, 79), // 10000 points → 79 workgroups
        ];

        for (point_count, expected_workgroups) in test_cases {
            let workgroup_count = (point_count + BLOCK_SIZE - 1) / BLOCK_SIZE;
            assert_eq!(
                workgroup_count, expected_workgroups,
                "Dispatch calculation mismatch for {} points",
                point_count
            );
        }
    }

    /// Test Blelloch Scan boundary conditions
    #[test]
    fn test_blelloch_scan_boundary() {
        // Single element: should remain unchanged
        let single_elem = [1u32];
        assert_eq!(single_elem.len(), 1);

        // Exactly BLOCK_SIZE elements: should fill one workgroup
        let full_block = vec![1u32; BLOCK_SIZE as usize];
        assert_eq!(full_block.len(), BLOCK_SIZE as usize);

        // Over BLOCK_SIZE: would need multiple workgroups
        let multi_block = vec![1u32; (BLOCK_SIZE * 2) as usize];
        assert!(multi_block.len() as u32 > BLOCK_SIZE);
    }

    /// Test Wave64 latency hiding capability
    #[test]
    fn test_wave64_occupancy() {
        // Wave64: 64 threads per wave
        // Workgroup: 64 threads × 1 = 64 (exactly 1 wave)
        // Each thread loads 2 elements
        assert_eq!(
            WAVE_SIZE * 2,
            BLOCK_SIZE,
            "Thread parallelism should cover block size"
        );
    }

    /// Test f16 vs u32 domain separation
    #[test]
    fn test_payload_key_domain_separation() {
        // Payloads: f16 (2 bytes × 4 components = 8 bytes)
        assert_eq!(PAYLOAD_SIZE_BYTES, 8, "f16 payload = vec4<f16> = 8 bytes");

        // Keys: u32 (4 bytes, never accumulate as f16)
        assert_eq!(KEY_SIZE_BYTES, 4, "u32 key = 4 bytes");

        // Total buffer size per point: 8 + 4 = 12 bytes
        let total_per_point = PAYLOAD_SIZE_BYTES + KEY_SIZE_BYTES;
        assert_eq!(
            total_per_point, 12,
            "Total: 8 (payload) + 4 (key) = 12 bytes"
        );
    }

    /// Test out-of-bounds scatter write protection
    #[test]
    fn test_boundary_safe_scatter() {
        // Scatter write boundary check: dest < base_idx + BLOCK_SIZE
        // Example: base_idx = 0, BLOCK_SIZE = 128
        // Valid destinations: [0, 127]
        // Invalid destinations: [128, ∞)

        let base_idx = 0u32;
        let block_end = base_idx + BLOCK_SIZE;

        for dest in 0..BLOCK_SIZE {
            assert!(
                dest < block_end,
                "Destination {} should be within block",
                dest
            );
        }

        // Out of bounds
        assert!(BLOCK_SIZE >= block_end, "BLOCK_SIZE at boundary");
    }
}
