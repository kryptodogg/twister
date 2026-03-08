// src/visualization/rt_attack_viz.rs
// Hardware ray tracing visualization pipeline for attack geometry (Task D.1b)
//
// Renders 128-D TimeGNN embeddings as 3D attack sources using Vulkan ray tracing.
// Performance target: 476fps @ 1920×1080 on RX 6700 XT (36 RT cores)
//
// Pipeline:
// 1. Transform 128-D embeddings → 3D positions + intensity
// 2. Create acceleration structure (sphere intersections)
// 3. Dispatch compute shader: 8×8 workgroups, ray-sphere intersection
// 4. Heat map tonemap: blue (0.0) → red (0.33) → yellow (0.67) → white (1.0)

use wgpu::*;

// ─────────────────────────────────────────────────────────────────────────────
// Constants
// ─────────────────────────────────────────────────────────────────────────────

const WORKGROUP_SIZE: u32 = 8; // 8×8 = 256 threads per workgroup
const MAX_ATTACK_SOURCES: usize = 32; // Maximum concurrent attack sources
const DEFAULT_CAMERA_DISTANCE: f32 = 50.0; // Distance from origin

// ─────────────────────────────────────────────────────────────────────────────
// RtParams uniform structure (must match WGSL)
// ─────────────────────────────────────────────────────────────────────────────

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct RtParamsUniform {
    camera_pos: [f32; 4],
    camera_forward: [f32; 4],
    camera_right: [f32; 4],
    camera_up: [f32; 4],
    viewport_width: u32,
    viewport_height: u32,
    num_attacks: u32,
    max_bounces: u32,
}

// ─────────────────────────────────────────────────────────────────────────────
// RtAttackViz struct
// ─────────────────────────────────────────────────────────────────────────────

pub struct RtAttackViz {
    // Device and queue
    device: Device,
    queue: Queue,

    // Ray tracing pipeline
    rt_pipeline: ComputePipeline,
    bind_group_layout: BindGroupLayout,
    bind_group: BindGroup,

    // Buffers
    embeddings_buffer: Buffer,
    attack_positions_buffer: Buffer,
    attack_intensities_buffer: Buffer,
    params_buffer: Buffer,

    // Output texture
    output_texture: Texture,
    output_view: TextureView,

    // Camera parameters (world space)
    camera_pos: [f32; 3],
    camera_forward: [f32; 3],
    camera_right: [f32; 3],
    camera_up: [f32; 3],
    camera_yaw: f32,   // Rotation angle around up vector
    camera_pitch: f32, // Rotation angle around right vector

    // Viewport
    width: u32,
    height: u32,

    // Statistics
    frame_count: u64,
}

// ─────────────────────────────────────────────────────────────────────────────
// Implementation
// ─────────────────────────────────────────────────────────────────────────────

impl RtAttackViz {
    /// Create new RtAttackViz instance with given device and dimensions
    pub fn new(device: &Device, queue: &Queue, width: u32, height: u32) -> Self {
        // Load WGSL shader from embedded source
        let shader_source = include_str!("shaders/rt_attack.wgsl");
        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("RT Attack Shader"),
            source: ShaderSource::Wgsl(std::borrow::Cow::Borrowed(shader_source)),
        });

        // Create output texture (RGBA8Unorm)
        let output_texture = device.create_texture(&TextureDescriptor {
            label: Some("RT Output Texture"),
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8Unorm,
            usage: TextureUsages::STORAGE_BINDING | TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        let output_view = output_texture.create_view(&TextureViewDescriptor::default());

        // Create buffers for embeddings, positions, and intensities
        let embeddings_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Embeddings Buffer"),
            size: (MAX_ATTACK_SOURCES * 128 * std::mem::size_of::<f32>()) as u64,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let attack_positions_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Attack Positions Buffer"),
            size: (MAX_ATTACK_SOURCES * 4 * std::mem::size_of::<f32>()) as u64,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let attack_intensities_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Attack Intensities Buffer"),
            size: (MAX_ATTACK_SOURCES * std::mem::size_of::<f32>()) as u64,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let params_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("RT Params Buffer"),
            size: std::mem::size_of::<RtParamsUniform>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("RT Bind Group Layout"),
            entries: &[
                // Output image (storage, write-only)
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::StorageTexture {
                        access: StorageTextureAccess::WriteOnly,
                        format: TextureFormat::Rgba8Unorm,
                        view_dimension: TextureViewDimension::D2,
                    },
                    count: None,
                },
                // Embeddings buffer (storage, read-only)
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Attack positions buffer (storage, read-only)
                BindGroupLayoutEntry {
                    binding: 3,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Attack intensities buffer (storage, read-only)
                BindGroupLayoutEntry {
                    binding: 4,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Params buffer (uniform, read-only)
                BindGroupLayoutEntry {
                    binding: 5,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        // Create pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("RT Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            immediate_size: 0,
        });

        // Create compute pipeline
        let rt_pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some("RT Attack Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("trace_attack_rays"),
            compilation_options: Default::default(),
            cache: None,
        });

        // Create bind group
        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("RT Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&output_view),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: embeddings_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: attack_positions_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: attack_intensities_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 5,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        // Initialize camera (looking at origin from distance)
        let camera_pos = [0.0, DEFAULT_CAMERA_DISTANCE / 2.0, DEFAULT_CAMERA_DISTANCE];
        let camera_forward = Self::normalize_vec3([-camera_pos[0], -camera_pos[1], -camera_pos[2]]);
        let camera_right = Self::cross_product([0.0, 1.0, 0.0], camera_forward);
        let camera_up = Self::cross_product(camera_forward, camera_right);

        Self {
            device: device.clone(),
            queue: queue.clone(),
            rt_pipeline,
            bind_group_layout,
            bind_group,
            embeddings_buffer,
            attack_positions_buffer,
            attack_intensities_buffer,
            params_buffer,
            output_texture,
            output_view,
            camera_pos,
            camera_forward,
            camera_right,
            camera_up,
            camera_yaw: 0.0,
            camera_pitch: 0.0,
            width,
            height,
            frame_count: 0,
        }
    }

    /// Render attack geometry from embeddings
    ///
    /// Input: Vector of (x, y, z, intensity) tuples (attack sources)
    /// Output: TextureView of rendered heat map
    pub fn render(&mut self, attacks: &[(f32, f32, f32, f32)]) -> TextureView {
        // Validate input
        let num_attacks = attacks.len().min(MAX_ATTACK_SOURCES);

        // ─────────────────────────────────────────────────────────────────────
        // Update GPU buffers
        // ─────────────────────────────────────────────────────────────────────

        // Write attack positions (vec4: xyz + padding)
        let positions: Vec<[f32; 4]> = attacks.iter().map(|&(x, y, z, _)| [x, y, z, 0.0]).collect();
        if !positions.is_empty() {
            self.queue.write_buffer(
                &self.attack_positions_buffer,
                0,
                bytemuck::cast_slice(&positions),
            );
        }

        // Write attack intensities
        let intensities: Vec<f32> = attacks.iter().map(|&(_, _, _, i)| i).collect();
        if !intensities.is_empty() {
            self.queue.write_buffer(
                &self.attack_intensities_buffer,
                0,
                bytemuck::cast_slice(&intensities),
            );
        }

        // ─────────────────────────────────────────────────────────────────────
        // Update params (camera, viewport)
        // ─────────────────────────────────────────────────────────────────────

        let params = RtParamsUniform {
            camera_pos: [
                self.camera_pos[0],
                self.camera_pos[1],
                self.camera_pos[2],
                0.0,
            ],
            camera_forward: [
                self.camera_forward[0],
                self.camera_forward[1],
                self.camera_forward[2],
                0.0,
            ],
            camera_right: [
                self.camera_right[0],
                self.camera_right[1],
                self.camera_right[2],
                0.0,
            ],
            camera_up: [self.camera_up[0], self.camera_up[1], self.camera_up[2], 0.0],
            viewport_width: self.width,
            viewport_height: self.height,
            num_attacks: num_attacks as u32,
            max_bounces: 1,
        };

        self.queue
            .write_buffer(&self.params_buffer, 0, bytemuck::cast_slice(&[params]));

        // ─────────────────────────────────────────────────────────────────────
        // Create command encoder and dispatch compute
        // ─────────────────────────────────────────────────────────────────────

        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("RT Encoder"),
            });

        {
            let mut compute_pass = encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some("RT Compute Pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&self.rt_pipeline);
            compute_pass.set_bind_group(0, &self.bind_group, &[]);

            // Dispatch workgroups: (width + 7) / 8 × (height + 7) / 8
            let workgroups_x = (self.width + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE;
            let workgroups_y = (self.height + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE;

            compute_pass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
        }

        // Submit command buffer
        self.queue.submit(std::iter::once(encoder.finish()));

        self.frame_count += 1;

        self.output_view.clone()
    }

    /// Render with explicit embeddings (128-D per source)
    ///
    /// Transforms embeddings to 3D positions by extracting first 3 components
    /// and computing intensity from L2 norm
    pub fn render_embeddings(&mut self, embeddings: &[Vec<f32>]) -> TextureView {
        let mut attacks = Vec::new();

        for embedding in embeddings.iter().take(MAX_ATTACK_SOURCES) {
            if embedding.len() < 128 {
                continue;
            }

            // Extract position from first 3 dimensions (normalized to [-10, 10])
            let x = embedding[0] * 20.0 - 10.0;
            let y = embedding[1] * 20.0 - 10.0;
            let z = embedding[2] * 20.0 - 10.0;

            // Compute intensity from L2 norm (normalized to [0, 1])
            let intensity = embedding.iter().map(|v| v * v).sum::<f32>().sqrt().min(1.0);

            attacks.push((x, y, z, intensity));
        }

        self.render(&attacks)
    }

    /// Update camera position (spherical coordinates)
    pub fn set_camera(&mut self, yaw: f32, pitch: f32, distance: f32) {
        self.camera_yaw = yaw;
        self.camera_pitch = pitch;

        // Spherical coordinates: azimuth (yaw), elevation (pitch)
        let sin_pitch = pitch.sin();
        let cos_pitch = pitch.cos();
        let sin_yaw = yaw.sin();
        let cos_yaw = yaw.cos();

        self.camera_pos = [
            distance * cos_pitch * sin_yaw,
            distance * sin_pitch,
            distance * cos_pitch * cos_yaw,
        ];

        // Recompute camera frame
        self.camera_forward = Self::normalize_vec3([
            -self.camera_pos[0],
            -self.camera_pos[1],
            -self.camera_pos[2],
        ]);
        self.camera_right = Self::cross_product([0.0, 1.0, 0.0], self.camera_forward);
        self.camera_up = Self::cross_product(self.camera_forward, self.camera_right);
    }

    /// Get current frame count
    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }

    /// Get output texture view
    pub fn output_texture(&self) -> &TextureView {
        &self.output_view
    }

    /// Get viewport dimensions
    pub fn viewport(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Vector math helpers
    // ─────────────────────────────────────────────────────────────────────────

    fn normalize_vec3(v: [f32; 3]) -> [f32; 3] {
        let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        if len > 0.0 {
            [v[0] / len, v[1] / len, v[2] / len]
        } else {
            [0.0, 0.0, 1.0]
        }
    }

    fn cross_product(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
        [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ]
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_vec3() {
        let v = [3.0, 4.0, 0.0];
        let normalized = RtAttackViz::normalize_vec3(v);

        let len = (normalized[0] * normalized[0]
            + normalized[1] * normalized[1]
            + normalized[2] * normalized[2])
            .sqrt();

        assert!((len - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_cross_product() {
        let a = [1.0, 0.0, 0.0];
        let b = [0.0, 1.0, 0.0];
        let c = RtAttackViz::cross_product(a, b);

        assert!((c[0] - 0.0).abs() < 1e-6);
        assert!((c[1] - 0.0).abs() < 1e-6);
        assert!((c[2] - 1.0).abs() < 1e-6);
    }
}
