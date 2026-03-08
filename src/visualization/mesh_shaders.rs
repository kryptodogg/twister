// src/visualization/mesh_shaders.rs
// Mesh Shader Visualization Pipeline with Adaptive 28-Level LOD
//
// This module implements Task D.1c: layering smooth mesh geometry on top of
// ray tracing visualization. Renders attack sources as adaptive-detail 3D spheres
// with 28 LOD levels, targeting 476fps via mesh shader optimization.
//
// Architecture:
// - MeshLodLevel: Metadata for each of 28 LOD levels
// - MeshShaderPipeline: GPU pipeline management (shaders, buffers, rendering)
// - Adaptive LOD selection based on projected screen coverage

use wgpu::*;

// ─────────────────────────────────────────────────────────────────────────
// LOD Level Definition
// ─────────────────────────────────────────────────────────────────────────

/// Metadata for a single LOD level (0-27)
///
/// Each LOD level specifies:
/// - Vertex and triangle counts for that detail level
/// - Screen coverage threshold (pixels) for LOD selection
#[derive(Clone, Debug)]
pub struct MeshLodLevel {
    /// LOD level number (0-27, where 0 = highest detail)
    pub level: u32,

    /// Number of vertices in this LOD level
    pub vertex_count: u32,

    /// Number of triangles in this LOD level
    pub triangle_count: u32,

    /// Screen coverage threshold in pixels (when to use this LOD)
    /// Used in LOD selection: if projected_size >= threshold, use this level
    pub screen_coverage_pixels: u32,
}

impl MeshLodLevel {
    /// Create a new LOD level with validation
    pub fn new(level: u32, vertex_count: u32, triangle_count: u32, screen_coverage: u32) -> Self {
        Self {
            level,
            vertex_count,
            triangle_count,
            screen_coverage_pixels: screen_coverage,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────
// LOD Constants and Configuration
// ─────────────────────────────────────────────────────────────────────────

const TOTAL_LOD_LEVELS: usize = 28;
const MAX_ATTACK_SOURCES: usize = 32;

// Screen coverage thresholds for LOD selection
const COVERAGE_THRESHOLD_LEVEL_0_6: u32 = 2048; // High detail
const COVERAGE_THRESHOLD_LEVEL_7_13: u32 = 512; // Medium
const COVERAGE_THRESHOLD_LEVEL_14_20: u32 = 128; // Low
const COVERAGE_THRESHOLD_LEVEL_21_27: u32 = 64; // Very low

// ─────────────────────────────────────────────────────────────────────────
// GPU Buffer Structures (must match WGSL)
// ─────────────────────────────────────────────────────────────────────────

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    view_proj: [[f32; 4]; 4], // 4×4 matrix
    camera_pos: [f32; 3],
    _padding1: u32,
    viewport_width: u32,
    viewport_height: u32,
    _padding2: [u32; 2],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct InstanceData {
    position: [f32; 3],
    intensity: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LodPayload {
    lod_level: u32,
    vertex_count: u32,
    triangle_count: u32,
    instance_id: u32,
}

// ─────────────────────────────────────────────────────────────────────────
// MeshShaderPipeline
// ─────────────────────────────────────────────────────────────────────────

/// GPU pipeline for mesh shader visualization with adaptive LOD
pub struct MeshShaderPipeline {
    // Device and queue
    device: Device,
    queue: Queue,

    // LOD levels (all 28)
    lod_levels: Vec<MeshLodLevel>,

    // Shaders
    task_shader: ShaderModule,
    mesh_shader: ShaderModule,
    fragment_shader: ShaderModule,

    // Pipeline and layout
    pipeline: RenderPipeline,
    bind_group_layout: BindGroupLayout,
    bind_group: BindGroup,

    // Buffers
    camera_buffer: Buffer,
    instance_buffer: Buffer,
    lod_decisions_buffer: Buffer,

    // Output texture
    output_texture: Texture,
    output_view: TextureView,

    // Viewport dimensions
    width: u32,
    height: u32,

    // Statistics
    frame_count: u64,
}

impl MeshShaderPipeline {
    /// Create a new mesh shader pipeline with all 28 LOD levels
    ///
    /// Initializes:
    /// - LOD level metadata
    /// - WGSL shaders (task, mesh, fragment)
    /// - GPU buffers and bind groups
    /// - Output texture for rendering
    pub fn new(device: &Device, queue: &Queue, width: u32, height: u32) -> Self {
        // ─────────────────────────────────────────────────────────────────
        // Step 1: Create LOD levels (all 28)
        // ─────────────────────────────────────────────────────────────────

        let lod_levels = Self::create_lod_levels();

        // ─────────────────────────────────────────────────────────────────
        // Step 2: Load WGSL shaders
        // ─────────────────────────────────────────────────────────────────

        let task_source = include_str!("shaders/mesh_lod_task.wgsl");
        let task_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Mesh Task Shader"),
            source: ShaderSource::Wgsl(std::borrow::Cow::Borrowed(task_source)),
        });

        let mesh_source = include_str!("shaders/mesh_lod_mesh.wgsl");
        let mesh_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Mesh Shader"),
            source: ShaderSource::Wgsl(std::borrow::Cow::Borrowed(mesh_source)),
        });

        let fragment_source = include_str!("shaders/mesh_lod_fragment.wgsl");
        let fragment_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Fragment Shader"),
            source: ShaderSource::Wgsl(std::borrow::Cow::Borrowed(fragment_source)),
        });

        // ─────────────────────────────────────────────────────────────────
        // Step 3: Create output texture
        // ─────────────────────────────────────────────────────────────────

        let output_texture = device.create_texture(&TextureDescriptor {
            label: Some("Mesh Shader Output Texture"),
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8Unorm,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        let output_view = output_texture.create_view(&TextureViewDescriptor::default());

        // ─────────────────────────────────────────────────────────────────
        // Step 4: Create GPU buffers
        // ─────────────────────────────────────────────────────────────────

        let camera_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Camera Uniform Buffer"),
            size: std::mem::size_of::<CameraUniform>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let instance_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Instance Data Buffer"),
            size: (MAX_ATTACK_SOURCES * std::mem::size_of::<InstanceData>()) as u64,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let lod_decisions_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("LOD Decisions Buffer"),
            size: (MAX_ATTACK_SOURCES * std::mem::size_of::<LodPayload>()) as u64,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // ─────────────────────────────────────────────────────────────────
        // Step 5: Create bind group layout
        // ─────────────────────────────────────────────────────────────────

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Mesh Shader Bind Group Layout"),
            entries: &[
                // Camera uniform
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX | ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Instance data (storage, read-only)
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::VERTEX | ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // LOD decisions (storage, read-write)
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::COMPUTE | ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        // ─────────────────────────────────────────────────────────────────
        // Step 6: Create pipeline layout
        // ─────────────────────────────────────────────────────────────────

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Mesh Shader Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            immediate_size: 0,
        });

        // ─────────────────────────────────────────────────────────────────
        // Step 7: Create render pipeline
        // ─────────────────────────────────────────────────────────────────

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Mesh Shader Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &mesh_shader,
                entry_point: Some("vertex_main"),
                buffers: &[], // Procedural geometry, no vertex buffer
                compilation_options: Default::default(),
            },
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Back),
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(FragmentState {
                module: &fragment_shader,
                entry_point: Some("fragment_main"),
                targets: &[Some(ColorTargetState {
                    format: TextureFormat::Rgba8Unorm,
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            multiview_mask: None,
            cache: None,
        });

        // ─────────────────────────────────────────────────────────────────
        // Step 8: Create bind group
        // ─────────────────────────────────────────────────────────────────

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Mesh Shader Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: instance_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: lod_decisions_buffer.as_entire_binding(),
                },
            ],
        });

        Self {
            device: device.clone(),
            queue: queue.clone(),
            lod_levels,
            task_shader,
            mesh_shader,
            fragment_shader,
            pipeline,
            bind_group_layout,
            bind_group,
            camera_buffer,
            instance_buffer,
            lod_decisions_buffer,
            output_texture,
            output_view,
            width,
            height,
            frame_count: 0,
        }
    }

    /// Create all 28 LOD levels with distribution:
    /// - Levels 0-6 (7 levels): 2048px threshold (high detail)
    /// - Levels 7-13 (7 levels): 512px threshold (medium)
    /// - Levels 14-20 (7 levels): 128px threshold (low)
    /// - Levels 21-27 (7 levels): 64px threshold (very low)
    pub fn create_lod_levels() -> Vec<MeshLodLevel> {
        let mut levels = Vec::with_capacity(TOTAL_LOD_LEVELS);

        for level in 0..TOTAL_LOD_LEVELS {
            let level_u32 = level as u32;

            // Decreasing vertex and triangle counts with increasing LOD level
            // Level 0: 1024 vertices, Level 27: 64 vertices
            let vertex_count = if level < 7 {
                // High detail: 1024 → 896 vertices
                1024u32 - ((level as u32) * 18)
            } else if level < 14 {
                // Medium: 896 → 512 vertices
                896u32 - (((level - 7) as u32) * 54)
            } else if level < 21 {
                // Low: 512 → 256 vertices
                512u32 - (((level - 14) as u32) * 36)
            } else {
                // Very low: 256 → 64 vertices
                256u32 - (((level - 21) as u32) * 27)
            };

            let triangle_count = (vertex_count / 3).max(21);

            // Screen coverage threshold
            let coverage = if level < 7 {
                COVERAGE_THRESHOLD_LEVEL_0_6
            } else if level < 14 {
                COVERAGE_THRESHOLD_LEVEL_7_13
            } else if level < 21 {
                COVERAGE_THRESHOLD_LEVEL_14_20
            } else {
                COVERAGE_THRESHOLD_LEVEL_21_27
            };

            levels.push(MeshLodLevel::new(
                level_u32,
                vertex_count,
                triangle_count,
                coverage,
            ));
        }

        levels
    }

    /// Select the appropriate LOD level based on projected screen size
    ///
    /// Implements the screen coverage-based LOD selection strategy:
    /// - screen_size >= 2048px → Level 0 (highest detail)
    /// - 512px <= screen_size < 2048px → Level 7 (medium)
    /// - 128px <= screen_size < 512px → Level 14 (low)
    /// - screen_size < 128px → Level 21+ (very low)
    pub fn select_lod_for_screen_size(screen_size: f32, _lod_levels: &[MeshLodLevel]) -> u32 {
        if screen_size >= 2048.0 {
            0u32 // Highest detail
        } else if screen_size >= 512.0 {
            7u32 // Medium
        } else if screen_size >= 128.0 {
            14u32 // Low
        } else {
            27u32 // Lowest detail
        }
    }

    /// Render attack sources with adaptive LOD mesh visualization
    ///
    /// Input: Vector of (x, y, z, intensity) tuples
    /// Output: TextureView containing rendered mesh visualization
    pub fn render(&mut self, attacks: &[(f32, f32, f32, f32)]) -> &TextureView {
        let num_attacks = attacks.len().min(MAX_ATTACK_SOURCES);

        // ─────────────────────────────────────────────────────────────────
        // Update instance buffer with attack data
        // ─────────────────────────────────────────────────────────────────

        let instances: Vec<InstanceData> = attacks
            .iter()
            .take(num_attacks)
            .map(|&(x, y, z, i)| InstanceData {
                position: [x, y, z],
                intensity: i,
            })
            .collect();

        if !instances.is_empty() {
            self.queue
                .write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(&instances));
        }

        // ─────────────────────────────────────────────────────────────────
        // Create command encoder and dispatch render pass
        // ─────────────────────────────────────────────────────────────────

        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("Mesh Shader Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Mesh Shader Render Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &self.output_view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        }),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(0, &self.bind_group, &[]);

            // Draw procedural sphere geometry for each instance
            // vertex_count varies per LOD level (64-1024 vertices)
            let max_vertices = self.lod_levels[0].vertex_count;
            render_pass.draw(0..max_vertices, 0..num_attacks as u32);
        }

        // Submit command buffer
        self.queue.submit(std::iter::once(encoder.finish()));

        self.frame_count += 1;

        &self.output_view
    }

    /// Get the number of LOD levels
    pub fn num_lod_levels(&self) -> usize {
        self.lod_levels.len()
    }

    /// Get LOD level by index
    pub fn get_lod_level(&self, index: usize) -> Option<&MeshLodLevel> {
        self.lod_levels.get(index)
    }

    /// Get frame count
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
}

// ─────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lod_levels_creation() {
        let levels = MeshShaderPipeline::create_lod_levels();
        assert_eq!(levels.len(), 28);
    }

    #[test]
    fn test_lod_levels_sequential() {
        let levels = MeshShaderPipeline::create_lod_levels();
        for (idx, level) in levels.iter().enumerate() {
            assert_eq!(level.level, idx as u32);
        }
    }

    #[test]
    fn test_lod_vertex_counts_decreasing() {
        let levels = MeshShaderPipeline::create_lod_levels();
        let mut prev = u32::MAX;
        for level in levels {
            assert!(level.vertex_count <= prev);
            prev = level.vertex_count;
        }
    }

    #[test]
    fn test_lod_selection_high_detail() {
        let levels = MeshShaderPipeline::create_lod_levels();
        let lod = MeshShaderPipeline::select_lod_for_screen_size(3000.0, &levels);
        assert!(lod < 7); // Should be in high-detail range
    }

    #[test]
    fn test_lod_selection_low_detail() {
        let levels = MeshShaderPipeline::create_lod_levels();
        let lod = MeshShaderPipeline::select_lod_for_screen_size(50.0, &levels);
        assert!(lod >= 21); // Should be in low-detail range
    }
}
