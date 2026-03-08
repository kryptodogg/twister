//! GPU Gaussian Splatting Renderer (Struct-of-Arrays optimization)
//!
//! Implements efficient 3D point cloud rendering using Gaussian splatting
//! with RDNA2-optimized Struct-of-Arrays (SoA) memory layout.
//!
//! Each particle attribute (azimuth, elevation, frequency, etc.) is stored in
//! a separate storage buffer to maximize memory coalescence on GPU.

use bytemuck::{Pod, Zeroable};
use std::mem::size_of;
use wgpu::*;

/// Single particle in the point cloud
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct Particle {
    pub azimuth_rad: f32,   // [0, 2π]
    pub elevation_rad: f32, // [-π/2, π/2]
    pub frequency_hz: f32,  // Detection frequency
    pub intensity: f32,     // Anomaly score [0, 1]
}

/// Configuration for gaussian splatting renderer
#[derive(Clone, Debug)]
pub struct GaussianSplattingConfig {
    /// Maximum particles to render
    pub max_particles: u32,
    /// Output texture width
    pub width: u32,
    /// Output texture height
    pub height: u32,
    /// Gaussian kernel standard deviation
    pub sigma: f32,
}

impl Default for GaussianSplattingConfig {
    fn default() -> Self {
        Self {
            max_particles: 10_000,
            width: 1024,
            height: 1024,
            sigma: 1.0,
        }
    }
}

/// GPU Gaussian Splatting Renderer
///
/// Stores particle data as 5 separate storage buffers (Struct-of-Arrays layout):
/// - Binding 0: azimuth (array<f32>)
/// - Binding 1: elevation (array<f32>)
/// - Binding 2: frequency (array<f32>)
/// - Binding 3: grid_hash (array<u32>) - for spatial hashing
/// - Binding 4: sorted_idx (array<u32>) - for sorted access
pub struct GaussianSplattingRenderer {
    device: Device,
    queue: Queue,
    config: GaussianSplattingConfig,

    // SoA buffers (separate storage buffer per attribute)
    azimuth_buffer: Buffer,
    elevation_buffer: Buffer,
    frequency_buffer: Buffer,
    grid_hash_buffer: Buffer,
    sorted_idx_buffer: Buffer,

    // GPU bindings
    bind_group_layout: BindGroupLayout,
    bind_group: BindGroup,

    // Compute pipeline for splatting
    compute_pipeline: ComputePipeline,

    // Output texture
    output_texture: Texture,
    output_texture_view: TextureView,

    // Staging buffer for readback
    staging_buffer: Buffer,

    // GPU timestamp query for performance measurement
    query_set: QuerySet,
    query_resolve_buffer: Buffer,
    query_readback_buffer: Buffer,
}

impl GaussianSplattingRenderer {
    /// Create a new Gaussian splatting renderer
    pub fn new(
        device: &Device,
        queue: &Queue,
        config: Option<GaussianSplattingConfig>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let config = config.unwrap_or_default();

        // Calculate buffer sizes with 256-byte alignment for RDNA2 Wave64 coalescing
        let particle_count = config.max_particles as usize;

        // Use environment variable to test different alignments
        let alignment_bytes = std::env::var("GSPLAT_ALIGNMENT")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(256);

        let aligned_size = ((particle_count as u64 * size_of::<f32>() as u64)
            + (alignment_bytes - 1))
            / alignment_bytes
            * alignment_bytes;
        let aligned_size = aligned_size as usize;

        eprintln!(
            "[GaussianSplat] Creating {} particles, {}-byte alignment ({} bytes total)",
            particle_count, alignment_bytes, aligned_size
        );

        // Create SoA storage buffers (SEPARATE bindings for each attribute)
        let azimuth_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("gaussian_splat_azimuth"),
            size: aligned_size as u64,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let elevation_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("gaussian_splat_elevation"),
            size: aligned_size as u64,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let frequency_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("gaussian_splat_frequency"),
            size: aligned_size as u64,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let grid_hash_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("gaussian_splat_grid_hash"),
            size: aligned_size as u64,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let sorted_idx_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("gaussian_splat_sorted_idx"),
            size: aligned_size as u64,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Create bind group layout with 5 storage buffer entries
        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("gaussian_splat_soa_layout"),
            entries: &[
                // Binding 0: azimuth
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Binding 1: elevation
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Binding 2: frequency
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Binding 3: grid_hash
                BindGroupLayoutEntry {
                    binding: 3,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Binding 4: sorted_idx
                BindGroupLayoutEntry {
                    binding: 4,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        // Create bind group linking buffers to layout
        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("gaussian_splat_soa_bindgroup"),
            layout: &bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: azimuth_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: elevation_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: frequency_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: grid_hash_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: sorted_idx_buffer.as_entire_binding(),
                },
            ],
        });

        // Create compute pipeline - select shader variant based on environment variable
        let wave_variant = std::env::var("GSPLAT_WAVE").unwrap_or_else(|_| "64".to_string());
        let is_wave32 = wave_variant == "32";

        let (shader_source, shader_label) = if is_wave32 {
            (
                include_str!("shaders/gaussian_splatting_wave32.wgsl"),
                "gaussian_splat_wave32",
            )
        } else {
            (
                include_str!("shaders/gaussian_splatting.wgsl"),
                "gaussian_splat_wave64",
            )
        };

        eprintln!(
            "[GaussianSplat] Using {} shader (Wave{} execution)",
            shader_label, wave_variant
        );

        let shader_module = device.create_shader_module(ShaderModuleDescriptor {
            label: Some(shader_label),
            source: ShaderSource::Wgsl(std::borrow::Cow::Borrowed(shader_source)),
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("gaussian_splat_layout"),
            bind_group_layouts: &[&bind_group_layout],
            immediate_size: 0,
        });

        let compute_pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some("gaussian_splat_compute"),
            layout: Some(&pipeline_layout),
            module: &shader_module,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        // Create output texture
        let output_texture = device.create_texture(&TextureDescriptor {
            label: Some("gaussian_splat_output"),
            size: Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8UnormSrgb,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        let output_texture_view = output_texture.create_view(&TextureViewDescriptor::default());

        // Create staging buffer for readback
        let staging_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("gaussian_splat_staging"),
            size: (config.width * config.height * 4) as u64,
            usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        // Create GPU timestamp query infrastructure (2 timestamps: start, end)
        let query_set = device.create_query_set(&QuerySetDescriptor {
            label: Some("gaussian_splat_timestamps"),
            ty: wgpu::QueryType::Timestamp,
            count: 2,
        });

        let query_resolve_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("gaussian_splat_query_resolve"),
            size: 16, // Two 64-bit timestamps
            usage: BufferUsages::QUERY_RESOLVE | BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let query_readback_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("gaussian_splat_query_readback"),
            size: 16,
            usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        eprintln!(
            "[GaussianSplat] Renderer initialized: {}x{}, 256-byte alignment, timestamps enabled",
            config.width, config.height
        );

        Ok(Self {
            device: device.clone(),
            queue: queue.clone(),
            config,
            azimuth_buffer,
            elevation_buffer,
            frequency_buffer,
            grid_hash_buffer,
            sorted_idx_buffer,
            bind_group_layout,
            bind_group,
            compute_pipeline,
            output_texture,
            output_texture_view,
            staging_buffer,
            query_set,
            query_resolve_buffer,
            query_readback_buffer,
        })
    }

    /// Upload particle data to GPU
    pub fn upload_particles(
        &self,
        particles: &[Particle],
    ) -> Result<(), Box<dyn std::error::Error>> {
        if particles.is_empty() {
            return Ok(());
        }

        if particles.len() > self.config.max_particles as usize {
            return Err(format!(
                "Too many particles: {} > {}",
                particles.len(),
                self.config.max_particles
            )
            .into());
        }

        // Convert to SoA format and upload each buffer separately
        let azimuths: Vec<f32> = particles.iter().map(|p| p.azimuth_rad).collect();
        let elevations: Vec<f32> = particles.iter().map(|p| p.elevation_rad).collect();
        let frequencies: Vec<f32> = particles.iter().map(|p| p.frequency_hz).collect();

        self.queue
            .write_buffer(&self.azimuth_buffer, 0, bytemuck::cast_slice(&azimuths));
        self.queue
            .write_buffer(&self.elevation_buffer, 0, bytemuck::cast_slice(&elevations));
        self.queue.write_buffer(
            &self.frequency_buffer,
            0,
            bytemuck::cast_slice(&frequencies),
        );

        eprintln!("[GaussianSplat] Uploaded {} particles", particles.len());

        Ok(())
    }

    /// Render gaussian splatting with GPU timestamp measurement
    pub fn render(&self, particle_count: u32) -> Result<(), Box<dyn std::error::Error>> {
        if particle_count == 0 {
            return Ok(());
        }

        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("gaussian_splat_encoder"),
            });

        {
            let mut cpass = encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some("gaussian_splat_pass"),
                timestamp_writes: Some(wgpu::ComputePassTimestampWrites {
                    query_set: &self.query_set,
                    beginning_of_pass_write_index: Some(0),
                    end_of_pass_write_index: Some(1),
                }),
            });

            cpass.set_pipeline(&self.compute_pipeline);
            cpass.set_bind_group(0, &self.bind_group, &[]);

            // Dispatch workgroups: (16, 16, 1) per group
            let workgroup_x = (self.config.width + 15) / 16;
            let workgroup_y = (self.config.height + 15) / 16;
            cpass.dispatch_workgroups(workgroup_x, workgroup_y, 1);
        }

        // Resolve query set to buffer
        encoder.resolve_query_set(&self.query_set, 0..2, &self.query_resolve_buffer, 0);

        // Copy resolved timestamps to readback buffer
        encoder.copy_buffer_to_buffer(
            &self.query_resolve_buffer,
            0,
            &self.query_readback_buffer,
            0,
            16,
        );

        self.queue.submit(std::iter::once(encoder.finish()));

        // Read back and calculate GPU time
        let buffer_slice = self.query_readback_buffer.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });

        // Wait for buffer mapping
        let _ = self.device.poll(wgpu::PollType::wait_indefinitely());

        if let Ok(Ok(())) = rx.recv() {
            let data = buffer_slice.get_mapped_range();
            let timestamps: [u64; 2] = [
                u64::from_le_bytes([
                    data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7],
                ]),
                u64::from_le_bytes([
                    data[8], data[9], data[10], data[11], data[12], data[13], data[14], data[15],
                ]),
            ];
            drop(data);
            self.query_readback_buffer.unmap();

            let delta_ns = timestamps[1].saturating_sub(timestamps[0]);
            let timestamp_period = self.queue.get_timestamp_period();
            let gpu_time_ms = (delta_ns as f64) * (timestamp_period as f64) / 1_000_000.0;

            println!(
                "[GaussianSplat] {} particles: {:.3} ms GPU compute time",
                particle_count, gpu_time_ms
            );
        }

        eprintln!(
            "[GaussianSplat] Dispatched compute kernel for {} particles",
            particle_count
        );

        Ok(())
    }

    /// Get output texture view (for rendering to screen)
    pub fn output_texture_view(&self) -> &TextureView {
        &self.output_texture_view
    }

    /// Get configuration
    pub fn config(&self) -> &GaussianSplattingConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_particle_size() {
        assert_eq!(std::mem::size_of::<Particle>(), 16);
    }

    #[test]
    fn test_config_default() {
        let config = GaussianSplattingConfig::default();
        assert_eq!(config.max_particles, 10_000);
        assert_eq!(config.width, 1024);
        assert_eq!(config.height, 1024);
    }
}
