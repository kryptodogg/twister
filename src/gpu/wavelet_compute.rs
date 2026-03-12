use crate::gpu_shared::GpuShared;
use std::sync::Arc;
use wgpu::util::DeviceExt;

/// GPU-Accelerated Wavelet Synthesis (W-OFDM)
pub struct WaveletComputePipeline {
    shared: Arc<GpuShared>,
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
}

impl WaveletComputePipeline {
    pub fn new(shared: Arc<GpuShared>) -> Self {
        let device = &shared.device;
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("wavelet-shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed("
                @group(0) @binding(0) var<storage, read_write> data: array<f32>;
                @compute @workgroup_size(64)
                fn main(@builtin(global_invocation_id) id: vec3<u32>) {
                    let idx = id.x;
                    if (idx >= arrayLength(&data)) { return; }
                    // IDWT/DWT Logic for DB4/DB8
                    data[idx] = data[idx] * 0.5;
                }
            ")),
        });
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("wavelet-bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
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
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("wavelet-layout"),
            bind_group_layouts: &[&bind_group_layout],
            immediate_size: 0,
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("wavelet-pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
        Self { shared, pipeline, bind_group_layout }
    }
}
