// src/gpu.rs
// Orchestrates the STFT + Coherence compute passes on the GPU.

use crate::gpu_shared::GpuShared;
use crate::vbuffer::{IqVBuffer, GpuVBuffer, VBufferPushConst, V_FREQ_BINS, V_DEPTH};
use std::sync::Arc;
use wgpu::util::DeviceExt;

pub struct SignalProcessorGpu {
    shared: Arc<GpuShared>,
    stft_pipeline: wgpu::ComputePipeline,
    coherence_pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
}

impl SignalProcessorGpu {
    pub fn new(shared: Arc<GpuShared>) -> Self {
        let device = &shared.device;

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("signal_processor_bgl"),
            entries: &[
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
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("signal_processor_layout"),
            bind_group_layouts: &[&bind_group_layout],
            immediate_size: std::mem::size_of::<VBufferPushConst>() as u32,
        });

        let stft_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("stft_iq.wgsl"),
            source: wgpu::ShaderSource::Wgsl(include_str!("visualization/shaders/stft_iq.wgsl").into()),
        });

        let coherence_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("coherence.wgsl"),
            source: wgpu::ShaderSource::Wgsl(include_str!("visualization/shaders/coherence.wgsl").into()),
        });

        let stft_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("stft_compute"),
            layout: Some(&pipeline_layout),
            module: &stft_shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        let coherence_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("coherence_compute"),
            layout: Some(&pipeline_layout),
            module: &coherence_shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        Self {
            shared,
            stft_pipeline,
            coherence_pipeline,
            bind_group_layout,
        }
    }

    pub fn process_frame(
        &self,
        iq_buffer: &IqVBuffer,
        gpu_vbuffer: &mut GpuVBuffer,
        context_len: u32,
    ) {
        let device = &self.shared.device;
        let queue = &self.shared.queue;

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("signal_processor_bg"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: iq_buffer.buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: gpu_vbuffer.buffer.as_entire_binding(),
                },
            ],
        });

        let pc = gpu_vbuffer.push_const(context_len);
        let pc_bytes = bytemuck::bytes_of(&pc);

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("signal_processor_encoder"),
        });

        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("stft_iq_pass"),
                timestamp_writes: None,
            });
            cpass.set_pipeline(&self.stft_pipeline);
            cpass.set_bind_group(0, &bind_group, &[]);
            cpass.set_immediates(0, pc_bytes);

            // Dispatch 64 threads for 512 bins (64 * 8 items = 512)
            cpass.dispatch_workgroups(1, 1, 1);
        }

        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("coherence_pass"),
                timestamp_writes: None,
            });
            cpass.set_pipeline(&self.coherence_pipeline);
            cpass.set_bind_group(0, &bind_group, &[]);
            cpass.set_immediates(0, pc_bytes);

            // Dispatch 512 workgroups (1 per freq bin)
            cpass.dispatch_workgroups(V_FREQ_BINS as u32, 1, 1);
        }

        queue.submit(std::iter::once(encoder.finish()));
        gpu_vbuffer.meta.version += 1; // Mark frame as pushed
    }
}
// Added process_frame and SignalProcessorGpu to gpu.rs
