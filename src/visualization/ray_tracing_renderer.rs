use std::error::Error;
use crate::visualization::data_contracts::VoxelGrid;
use std::num::NonZeroU32;

#[derive(Debug, Clone, Copy)]
pub struct Camera {
    pub position: [f32; 3],
    pub view_matrix: [[f32; 4]; 4],
}

impl Camera {
    pub fn new() -> Self {
        Self {
            position: [0.0, 0.0, 0.0],
            view_matrix: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }
}

pub struct BVH {}

impl BVH {
    pub fn from_room_geometry(_geometry: &[f32]) -> Self {
        Self {}
    }
}

pub fn get_room_geometry() -> Vec<f32> {
    vec![0.0; 100]
}

pub struct RayTracingRenderer {
    device: wgpu::Device,
    _queue: wgpu::Queue,
    _rt_pipeline: wgpu::RenderPipeline,
    _bvh: BVH,
    _ray_generation_shader: wgpu::ShaderModule,
    _ray_closest_hit_shader: wgpu::ShaderModule,
    _ray_miss_shader: wgpu::ShaderModule,
}

impl RayTracingRenderer {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let ray_gen_shader = Self::load_ray_gen_shader(device);
        let ray_hit_shader = Self::load_hit_shader(device);
        let ray_miss_shader = Self::load_miss_shader(device);

        let rt_pipeline = Self::create_rt_pipeline(device, &ray_gen_shader);
        let bvh = BVH::from_room_geometry(&get_room_geometry());

        Self {
            device: device.clone(),
            _queue: queue.clone(),
            _rt_pipeline: rt_pipeline,
            _bvh: bvh,
            _ray_generation_shader: ray_gen_shader,
            _ray_closest_hit_shader: ray_hit_shader,
            _ray_miss_shader: ray_miss_shader,
        }
    }

    fn create_rt_pipeline(device: &wgpu::Device, shader: &wgpu::ShaderModule) -> wgpu::RenderPipeline {
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Dummy RT Pipeline Layout"),
            bind_group_layouts: &[],
            immediate_size: 0,

        });

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Ray Tracing Dummy Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),

            cache: None,
            multiview_mask: None,
        })
    }

    fn load_ray_gen_shader(device: &wgpu::Device) -> wgpu::ShaderModule {
        device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Ray Generation Shader"),
            source: wgpu::ShaderSource::Wgsl(RAY_GENERATION_SHADER.into()),
        })
    }

    fn load_hit_shader(device: &wgpu::Device) -> wgpu::ShaderModule {
        device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Ray Closest Hit Shader"),
            source: wgpu::ShaderSource::Wgsl(RAY_CLOSEST_HIT_SHADER.into()),
        })
    }

    fn load_miss_shader(device: &wgpu::Device) -> wgpu::ShaderModule {
        device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Ray Miss Shader"),
            source: wgpu::ShaderSource::Wgsl(RAY_MISS_SHADER.into()),
        })
    }

    pub fn render_with_ray_tracing(
        &self,
        _energy_field: &VoxelGrid,
        _camera: &Camera,
        viewport_size: (u32, u32),
    ) -> Result<wgpu::Texture, Box<dyn Error>> {
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("RT Output Texture"),
            size: wgpu::Extent3d {
                width: viewport_size.0,
                height: viewport_size.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        Ok(texture)
    }

    pub fn render_megalights(&self) -> wgpu::CommandBuffer {
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Megalights Command Encoder"),
        });

        encoder.finish()
    }
}

pub const RAY_GENERATION_SHADER: &str = r#"
@vertex fn vs_main() -> @builtin(position) vec4<f32> {
    return vec4<f32>(0.0, 0.0, 0.0, 1.0);
}
@fragment fn fs_main() -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 0.0, 0.0, 1.0);
}
"#;

pub const RAY_CLOSEST_HIT_SHADER: &str = r#"
"#;

pub const RAY_MISS_SHADER: &str = r#"
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bvh_construction() {
        let geom = get_room_geometry();
        let _bvh = BVH::from_room_geometry(&geom);
    }
    #[test] fn test_ray_generation() {}
    #[test] fn test_intersection_computation() { }
    #[test] fn test_energy_field_sampling() { }
    #[test] fn test_ray_tracing_10m_particles() { }
}
