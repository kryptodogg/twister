use crate::gpu_shared::GpuShared;
use crate::particle_system::frustum_culler::FrustumCuller;
use std::sync::Arc;
use wgpu::util::DeviceExt;

pub struct ParticleRenderer {
    shared: Arc<GpuShared>,
    pub particle_buffer: wgpu::Buffer,
    mesh_shader_pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
}

impl ParticleRenderer {
    pub fn new(
        shared: Arc<GpuShared>,
        max_particles: u32,
        target_format: wgpu::TextureFormat,
    ) -> Self {
        let device = &shared.device;

        let particle_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("particle_data_buffer"),
            size: (max_particles as u64) * 44,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("particle_mesh_shader_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None, // Frustum
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None, // Particles
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None, // Visible Indices
                },
            ],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("particle_mesh_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/particle_mesh.wgsl").into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("particle_mesh_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            immediate_size: 0,
        });

        let mesh_shader_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("particle_mesh_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: target_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });

        Self {
            shared,
            particle_buffer,
            mesh_shader_pipeline,
            bind_group_layout,
        }
    }

    pub fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>, culler: &'a FrustumCuller) {
        let device = &self.shared.device;

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("particle_mesh_bg"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: culler.frustum_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.particle_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: culler.cull_buffer.as_entire_binding(), // Assuming we renamed cull buffer or make it public
                },
            ],
        });

        render_pass.set_pipeline(&self.mesh_shader_pipeline);
        render_pass.set_bind_group(0, &bind_group, &[]);

        // Instanced draw call using indirect arguments. Instance count is driven by the compute shader.
        // The vertex shader will pull the particle data based on the instance_index -> visible_indices -> raw_particles.
        render_pass.draw_indirect(&culler.draw_indirect_buffer, 0);
    }
}
