// src/visualization/stft_pipeline.rs — GPU STFT Compute Pipeline
//
// Orchestrates the Short-Time Fourier Transform pipeline:
//   IQ Buffer → FFT Shader → Magnitude Output → V-Buffer
//
// Uses the WGSL shader at src/visualization/shaders/stft_iq.wgsl
// Workgroup size: 64 threads (Wave64 optimized for RDNA2)

use crate::vbuffer::GpuVBuffer;
use std::sync::Arc;

/// Number of IQ samples per FFT frame (must match shader constant).
pub const FFT_SIZE: usize = 512;

/// Output frequency bins (512 bins after FFT).
pub const FREQ_BINS: usize = 512;

/// GPU compute pipeline for STFT processing.
pub struct StftPipeline {
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
    pub pipeline: wgpu::ComputePipeline,
    pub bind_group_layout: wgpu::BindGroupLayout,
}

impl StftPipeline {
    /// Create a new STFT pipeline from WGSL shader.
    ///
    /// # Parameters
    /// - `device`: wgpu Device for pipeline creation
    /// - `queue`: wgpu Queue for command submission
    ///
    /// # Returns
    /// Initialized STFT compute pipeline ready for dispatch
    pub fn new(device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>) -> Result<Self, String> {
        // Load the WGSL shader
        let shader_source = include_str!("shaders/stft_iq.wgsl");
        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("STFT IQ Shader"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        // Create bind group layout (must match shader bindings)
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("STFT Bind Group Layout"),
            entries: &[
                // raw_iq: storage buffer (read-only)
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
                // vbuffer: storage buffer (read-write)
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

        // Create pipeline layout (no push constants - using bind group only)
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("STFT Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            immediate_size: 0,
        });

        // Create compute pipeline
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("STFT Compute Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader_module,
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        Ok(StftPipeline {
            device,
            queue,
            pipeline,
            bind_group_layout,
        })
    }

    /// Dispatch STFT on IQ data.
    ///
    /// # Parameters
    /// - `iq_buffer`: GPU buffer containing raw IQ samples ([i8; 2] × 512)
    /// - `vbuffer`: Destination V-buffer for magnitude/phase output
    ///
    /// # Behavior
    /// 1. Creates bind group from IQ buffer and V-buffer
    /// 2. Encodes compute dispatch (8 workgroups × 64 threads = 512 threads)
    /// 3. Submits to GPU queue
    pub fn dispatch(
        &self,
        iq_buffer: &wgpu::Buffer,
        vbuffer: &mut GpuVBuffer,
    ) -> Result<(), String> {
        // Create bind group
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("STFT Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: iq_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: vbuffer.buffer.as_entire_binding(),
                },
            ],
        });

        // Encode compute commands
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("STFT Dispatch Encoder"),
            });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("STFT Compute Pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&self.pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);

            // Dispatch 8 workgroups (8 × 64 threads = 512 total)
            compute_pass.dispatch_workgroups(8, 1, 1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));

        Ok(())
    }
}

/// High-level STFT processor that combines pipeline with buffer management.
pub struct StftProcessor {
    pub pipeline: StftPipeline,
    pub iq_buffer: wgpu::Buffer,
    pub output_vbuffer: GpuVBuffer,
}

impl StftProcessor {
    /// Create a new STFT processor.
    ///
    /// # Parameters
    /// - `device`: wgpu Device
    /// - `queue`: wgpu Queue
    ///
    /// # Returns
    /// Initialized processor with allocated buffers
    pub fn new(device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>) -> Result<Self, String> {
        let pipeline = StftPipeline::new(Arc::clone(&device), Arc::clone(&queue))?;

        // Allocate IQ buffer (512 complex samples = 1024 bytes)
        let iq_buffer_size = (FFT_SIZE * 2) as u64;
        let iq_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("STFT IQ Input Buffer"),
            size: iq_buffer_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Allocate V-buffer for output
        let output_vbuffer = GpuVBuffer::new(&device);

        Ok(StftProcessor {
            pipeline,
            iq_buffer,
            output_vbuffer,
        })
    }

    /// Process one frame of IQ data.
    ///
    /// # Parameters
    /// - `iq_samples`: Raw IQ samples ([i8; 2] × 512)
    ///
    /// # Behavior
    /// 1. Uploads IQ samples to GPU buffer
    /// 2. Dispatches FFT shader
    /// 3. V-buffer automatically updated with magnitude/phase
    pub fn process_frame(&mut self, iq_samples: &[[i8; 2]]) -> Result<(), String> {
        // Upload IQ samples to GPU
        self.pipeline
            .queue
            .write_buffer(&self.iq_buffer, 0, bytemuck::cast_slice(&iq_samples[..]));

        // Dispatch FFT
        self.pipeline
            .dispatch(&self.iq_buffer, &mut self.output_vbuffer)?;

        Ok(())
    }

    /// Get the output V-buffer (for reading by downstream consumers).
    pub fn vbuffer(&self) -> &GpuVBuffer {
        &self.output_vbuffer
    }

    /// Get mutable V-buffer (for pushing to visualization).
    pub fn vbuffer_mut(&mut self) -> &mut GpuVBuffer {
        &mut self.output_vbuffer
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stft_constants() {
        assert_eq!(FFT_SIZE, 512);
        assert_eq!(FREQ_BINS, 512);
    }
}
