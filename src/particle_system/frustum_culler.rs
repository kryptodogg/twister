use std::sync::Arc;
use wgpu::util::DeviceExt;
use crate::gpu_shared::GpuShared;

pub struct FrustumCuller {
    shared: Arc<GpuShared>,
    pub frustum_buffer: wgpu::Buffer,
    pub cull_buffer: wgpu::Buffer,
    pub draw_indirect_buffer: wgpu::Buffer,
    cull_pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct FrustumUniform {
    pub view_proj: [f32; 16], // 4x4 standard matrix
    pub particle_count: u32,
    pub _padding: [u32; 3], // 16-byte align
}

impl FrustumCuller {
    pub fn new(shared: Arc<GpuShared>, max_particles: u32) -> Self {
        let device = &shared.device;

        let frustum_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("frustum_uniform_buffer"),
            size: std::mem::size_of::<FrustumUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // The compute shader needs a storage buffer for cull decisions
        // Though mesh shaders can cull dynamically, the spec asks for "atomically decrements the draw count and culls the particle."
        // We'll prepare an indirect draw buffer: { vertex_count, instance_count, base_vertex, base_instance } or mesh payload.
        // Wait, mesh shaders use DispatchMeshIndirect: { task_count, first_task, first_task_y, first_task_z }
        // Let's actually create a buffer for visible indices if needed, or simply let the compute shader build the indirect args.
        let cull_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("cull_decisions_buffer"),
            size: (max_particles * 4) as u64, // u32 boolean per particle
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Indirect buffer for DrawMeshTasksIndirect
        // DrawMeshTasksIndirect { task_count_x: u32, task_count_y: u32, task_count_z: u32 } = 12 bytes
        // We align it to 16 bytes for padding
        let draw_indirect_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("mesh_indirect_buffer"),
            size: 16, // wgpu DrawMeshTasksIndirect requires 12 bytes (or 16 with padding)
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::INDIRECT | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("frustum_culler_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None, // Particles
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None, // Indirect Draw Args
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None, // Visible Index Mapping
                },
            ],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("frustum_culler_compute"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/frustum_culler.wgsl").into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("frustum_culler_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            immediate_size: 0,
        });

        let cull_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("frustum_culler_pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        Self {
            shared,
            frustum_buffer,
            cull_buffer,
            draw_indirect_buffer,
            cull_pipeline,
            bind_group_layout,
        }
    }

    pub fn cull(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        particle_buffer: &wgpu::Buffer,
        particle_count: u32,
        view_proj: [f32; 16],
    ) {
        let device = &self.shared.device;
        let queue = &self.shared.queue;

        // Reset indirect buffer: task_count_x = 0, y=1, z=1
        // (y and z remain 1 for DrawMeshTasksIndirect)
        let reset_indirect = [6u32, 0u32, 0u32, 0u32];
        queue.write_buffer(&self.draw_indirect_buffer, 0, bytemuck::cast_slice(&reset_indirect));

        let uniform = FrustumUniform {
            view_proj,
            particle_count,
            _padding: [0; 3],
        };
        queue.write_buffer(&self.frustum_buffer, 0, bytemuck::bytes_of(&uniform));

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("frustum_culler_bind_group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.frustum_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: particle_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.draw_indirect_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: self.cull_buffer.as_entire_binding(),
                },
            ],
        });

        let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("frustum_cull_pass"),
            timestamp_writes: None,
        });

        cpass.set_pipeline(&self.cull_pipeline);
        cpass.set_bind_group(0, &bind_group, &[]);

        let workgroup_size = 256;
        let workgroups = (particle_count + workgroup_size - 1) / workgroup_size;
        cpass.dispatch_workgroups(workgroups, 1, 1);
    }
}
