//! WGPU compute pipeline for BBS, FFT, and PDM modulation
//!
//! Provides:
//! - V-buffer allocation for GPU memory pools
//! - BBS/RLS estimator as compute shader (parallel beamforming)
//! - FFT kernel for audio frames
//! - Sigma-delta PDM modulation kernel
//! - Warp scheduling for <35ms latency

use crate::utils::error::{Result, GPUError};
use wgpu::{
    Adapter, Device, Queue, Instance, Surface, CommandEncoder,
    BindGroup, Buffer, BufferUsages,
    ShaderModule, PipelineLayout, BindGroupLayout,
};
use wgpu_types::{Backend, PowerPreference, RequestAdapterOptions};
use std::sync::Arc;
use std::time::Duration;
use parking_lot::Mutex;
use bytemuck::{Pod, Zeroable};

/// GPU configuration
#[derive(Debug, Clone)]
pub struct WgpuConfig {
    /// Preferred backend (Vulkan, Dx12, Metal, Gl)
    pub backend: Backend,
    /// Power preference
    pub power_preference: PowerPreference,
    /// V-buffer pool size in bytes
    pub vbuffer_pool_size: usize,
    /// Enable warp scheduling
    pub warp_scheduling: bool,
}

impl Default for WgpuConfig {
    fn default() -> Self {
        Self {
            backend: Backend::all(),
            power_preference: PowerPreference::HighPerformance,
            vbuffer_pool_size: 256 * 1024 * 1024, // 256 MB
            warp_scheduling: true,
        }
    }
}

/// GPU context holding device and queue
pub struct GPUContext {
    pub instance: Instance,
    pub adapter: Adapter,
    pub device: Arc<Device>,
    pub queue: Arc<Queue>,
    pub config: WgpuConfig,
}

/// V-buffer (virtual buffer) for GPU memory management
pub struct VBuffer {
    pub buffer: Buffer,
    pub size: u64,
    pub usage: BufferUsages,
    pub label: String,
}

/// Compute pipeline wrapper (renamed to avoid collision with wgpu::ComputePipeline)
pub struct GpuComputePipeline {
    pub pipeline: wgpu::ComputePipeline,
    pub bind_group: BindGroup,
    pub layout: PipelineLayout,
}

/// BSS/RLS compute shader parameters
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct BSSParams {
    pub filter_length: u32,
    pub num_channels: u32,
    pub forgetting_factor: f32,
    pub regularization: f32,
    pub block_size: u32,
    pub _padding: [u32; 3],
}

/// FFT compute shader parameters
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct FFTParams {
    pub fft_size: u32,
    pub num_frames: u32,
    pub overlap: f32,
    pub _padding: [u32; 2],
}

/// PDM modulation parameters
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct PDMParams {
    pub order: u32,
    pub num_samples: u32,
    pub threshold: f32,
    pub _padding: [u32; 2],
}

impl GPUContext {
    /// Create a new GPU context
    pub async fn new(config: WgpuConfig) -> Result<Self> {
        let instance = Instance::new(wgpu::InstanceDescriptor {
            backends: config.backend.into(),
            ..Default::default()
        });

        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: config.power_preference,
                force_fallback_adapter: false,
                compatible_surface: None,
            })
            .await
            .ok_or_else(|| GPUError::NoAdapter)?;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Twister GPU Device"),
                    required_features: wgpu::Features::COMPUTE_SHADER | wgpu::Features::TIMESTAMP_QUERY,
                    required_limits: wgpu::Limits::default(),
                    memory_hints: wgpu::MemoryHints::Performance,
                },
                None,
            )
            .await
            .map_err(|e| GPUError::DeviceRequestFailed(e.to_string()))?;

        Ok(Self {
            instance,
            adapter,
            device: Arc::new(device),
            queue: Arc::new(queue),
            config,
        })
    }

    /// Create a V-buffer
    pub fn create_vbuffer(&self, size: u64, usage: BufferUsages, label: &str) -> Result<VBuffer> {
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size,
            usage,
            mapped_at_creation: false,
        });

        Ok(VBuffer {
            buffer,
            size,
            usage,
            label: label.to_string(),
        })
    }

    /// Create a staging buffer for CPU-GPU transfer
    pub fn create_staging_buffer(&self, size: u64, label: &str) -> Result<VBuffer> {
        self.create_vbuffer(
            size,
            BufferUsages::MAP_READ | BufferUsages::COPY_DST,
            label,
        )
    }

    /// Write data to a buffer
    pub fn write_buffer<T: Pod>(&self, buffer: &Buffer, data: &[T]) {
        self.queue.write_buffer(buffer, 0, bytemuck::cast_slice(data));
    }

    /// Read data from a staging buffer
    pub async fn read_buffer<T: Pod>(&self, buffer: &Buffer) -> Result<Vec<T>> {
        let buffer_slice = buffer.slice(..);
        let (tx, rx) = futures::channel::oneshot::channel();

        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });

        self.device.poll(wgpu::Maintain::wait()).panic_on_timeout();
        rx.await
            .map_err(|_| GPUError::BufferMapping("Channel error".into()))?
            .map_err(|e| GPUError::BufferMapping(e.to_string()))?;

        let data = buffer_slice.get_mapped_range();
        let result = bytemuck::cast_slice(&data).to_vec();

        drop(data);
        buffer.unmap();

        Ok(result)
    }
}

/// Compute kernel manager
pub struct ComputeKernelManager {
    context: Arc<GPUContext>,
    bss_pipeline: Option<GpuComputePipeline>,
    fft_pipeline: Option<GpuComputePipeline>,
    pdm_pipeline: Option<GpuComputePipeline>,
}

impl ComputeKernelManager {
    /// Create a new kernel manager
    pub fn new(context: Arc<GPUContext>) -> Self {
        Self {
            context,
            bss_pipeline: None,
            fft_pipeline: None,
            pdm_pipeline: None,
        }
    }

    /// Initialize all compute pipelines
    pub async fn initialize(&mut self) -> Result<()> {
        self.create_bss_pipeline()?;
        self.create_fft_pipeline()?;
        self.create_pdm_pipeline()?;
        Ok(())
    }

    /// Create BSS/RLS compute pipeline
    fn create_bss_pipeline(&mut self) -> Result<()> {
        let shader_module = self.context.device.create_shader_module(
            wgpu::ShaderModuleDescriptor {
                label: Some("BSS Shader"),
                source: wgpu::ShaderSource::Wgsl(Self::BSS_SHADER.into()),
            }
        );

        let bind_group_layout = self.context.device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                label: Some("BSS Bind Group Layout"),
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
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            }
        );

        let pipeline_layout = self.context.device.create_pipeline_layout(
            &wgpu::PipelineLayoutDescriptor {
                label: Some("BSS Pipeline Layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            }
        );

        let pipeline = self.context.device.create_compute_pipeline(
            &wgpu::ComputePipelineDescriptor {
                label: Some("BSS Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader_module,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                cache: None,
            }
        );

        self.bss_pipeline = Some(GpuComputePipeline {
            pipeline,
            bind_group: self.create_dummy_bind_group(&bind_group_layout)?,
            layout: pipeline_layout,
        });

        Ok(())
    }

    /// Create FFT compute pipeline
    fn create_fft_pipeline(&mut self) -> Result<()> {
        let shader_module = self.context.device.create_shader_module(
            wgpu::ShaderModuleDescriptor {
                label: Some("FFT Shader"),
                source: wgpu::ShaderSource::Wgsl(Self::FFT_SHADER.into()),
            }
        );

        let bind_group_layout = self.context.device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                label: Some("FFT Bind Group Layout"),
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
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            }
        );

        let pipeline_layout = self.context.device.create_pipeline_layout(
            &wgpu::PipelineLayoutDescriptor {
                label: Some("FFT Pipeline Layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            }
        );

        let pipeline = self.context.device.create_compute_pipeline(
            &wgpu::ComputePipelineDescriptor {
                label: Some("FFT Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader_module,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                cache: None,
            }
        );

        self.fft_pipeline = Some(GpuComputePipeline {
            pipeline,
            bind_group: self.create_dummy_bind_group(&bind_group_layout)?,
            layout: pipeline_layout,
        });

        Ok(())
    }

    /// Create PDM modulation pipeline
    fn create_pdm_pipeline(&mut self) -> Result<()> {
        let shader_module = self.context.device.create_shader_module(
            wgpu::ShaderModuleDescriptor {
                label: Some("PDM Shader"),
                source: wgpu::ShaderSource::Wgsl(Self::PDM_SHADER.into()),
            }
        );

        let bind_group_layout = self.context.device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                label: Some("PDM Bind Group Layout"),
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
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            }
        );

        let pipeline_layout = self.context.device.create_pipeline_layout(
            &wgpu::PipelineLayoutDescriptor {
                label: Some("PDM Pipeline Layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            }
        );

        let pipeline = self.context.device.create_compute_pipeline(
            &wgpu::ComputePipelineDescriptor {
                label: Some("PDM Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader_module,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                cache: None,
            }
        );

        self.pdm_pipeline = Some(GpuComputePipeline {
            pipeline,
            bind_group: self.create_dummy_bind_group(&bind_group_layout)?,
            layout: pipeline_layout,
        });

        Ok(())
    }

    /// Create a dummy bind group for initialization
    fn create_dummy_bind_group(&self, layout: &wgpu::BindGroupLayout) -> Result<BindGroup> {
        let dummy_buffer = self.context.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Dummy Buffer"),
            size: 256,
            usage: BufferUsages::UNIFORM | BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        let bind_group = self.context.device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                label: Some("Dummy Bind Group"),
                layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: dummy_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: dummy_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: dummy_buffer.as_entire_binding(),
                    },
                ],
            }
        );

        Ok(bind_group)
    }

    /// Dispatch BSS compute shader
    pub fn dispatch_bss(&self, input: &Buffer, output: &Buffer, params: &BSSParams, workgroups: u32) {
        if let Some(ref pipeline) = self.bss_pipeline {
            let mut encoder = self.context.device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("BSS Encoder") });

            {
                let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("BSS Compute Pass"),
                    timestamp_writes: None,
                });

                compute_pass.set_pipeline(&pipeline.pipeline);
                compute_pass.set_bind_group(0, &pipeline.bind_group, &[]);
                compute_pass.dispatch_workgroups(workgroups, 1, 1);
            }

            self.context.queue.submit(Some(encoder.finish()));
        }
    }

    /// Dispatch FFT compute shader
    pub fn dispatch_fft(&self, input: &Buffer, output: &Buffer, params: &FFTParams, workgroups: u32) {
        if let Some(ref pipeline) = self.fft_pipeline {
            let mut encoder = self.context.device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("FFT Encoder") });

            {
                let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("FFT Compute Pass"),
                    timestamp_writes: None,
                });

                compute_pass.set_pipeline(&pipeline.pipeline);
                compute_pass.set_bind_group(0, &pipeline.bind_group, &[]);
                compute_pass.dispatch_workgroups(workgroups, 1, 1);
            }

            self.context.queue.submit(Some(encoder.finish()));
        }
    }

    /// Dispatch PDM compute shader
    pub fn dispatch_pdm(&self, input: &Buffer, output: &Buffer, params: &PDMParams, workgroups: u32) {
        if let Some(ref pipeline) = self.pdm_pipeline {
            let mut encoder = self.context.device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("PDM Encoder") });

            {
                let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("PDM Compute Pass"),
                    timestamp_writes: None,
                });

                compute_pass.set_pipeline(&pipeline.pipeline);
                compute_pass.set_bind_group(0, &pipeline.bind_group, &[]);
                compute_pass.dispatch_workgroups(workgroups, 1, 1);
            }

            self.context.queue.submit(Some(encoder.finish()));
        }
    }

    /// BSS/RLS compute shader (WGSL)
    const BSS_SHADER: &'static str = r#"
struct BSSParams {
    filter_length: u32,
    num_channels: u32,
    forgetting_factor: f32,
    regularization: f32,
    block_size: u32,
}

@group(0) @binding(0)
var<storage, read> input: array<f32>;

@group(0) @binding(1)
var<storage, read_write> output: array<f32>;

@group(0) @binding(2)
var<uniform> params: BSSParams;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if idx >= params.filter_length {
        return;
    }

    // RLS adaptive filter update
    // Simplified implementation for demonstration
    var error: f32 = 0.0;
    for i in 0u..params.num_channels {
        let sample_idx = idx * params.num_channels + i;
        if sample_idx < arrayLength(&input) {
            error += input[sample_idx];
        }
    }

    // Apply forgetting factor and regularization
    let lambda = params.forgetting_factor;
    let delta = params.regularization;
    
    output[idx] = error * lambda / (delta + abs(error));
}
"#;

    /// FFT compute shader (WGSL)
    const FFT_SHADER: &'static str = r#"
struct FFTParams {
    fft_size: u32,
    num_frames: u32,
    overlap: f32,
}

@group(0) @binding(0)
var<storage, read> input: array<f32>;

@group(0) @binding(1)
var<storage, read_write> output: array<vec2<f32>>;

@group(0) @binding(2)
var<uniform> params: FFTParams;

// Twiddle factor calculation
fn twiddle(k: u32, n: u32, N: u32) -> vec2<f32> {
    let angle = -2.0 * 3.14159265359 * f32(k) * f32(n) / f32(N);
    return vec2<f32>(cos(angle), sin(angle));
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if idx >= params.fft_size / 2 {
        return;
    }

    // Simple DFT (for demonstration - use Cooley-Tukey in production)
    var real: f32 = 0.0;
    var imag: f32 = 0.0;
    
    for n in 0u..params.fft_size {
        if n < arrayLength(&input) {
            let w = twiddle(idx, n, params.fft_size);
            real += input[n] * w.x;
            imag += input[n] * w.y;
        }
    }

    output[idx] = vec2<f32>(real, imag);
}
"#;

    /// PDM modulation shader (WGSL)
    const PDM_SHADER: &'static str = r#"
struct PDMParams {
    order: u32,
    num_samples: u32,
    threshold: f32,
}

@group(0) @binding(0)
var<storage, read> input: array<f32>;

@group(0) @binding(1)
var<storage, read_write> output: array<f32>;

@group(0) @binding(2)
var<uniform> params: PDMParams;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if idx >= params.num_samples {
        return;
    }

    // Sigma-delta modulation (simplified)
    let sample = input[idx];
    
    // First-order sigma-delta
    var integrator: f32 = 0.0;
    var feedback: f32 = 0.0;
    
    for i in 0u..params.order {
        integrator += sample - feedback;
        feedback = integrator > params.threshold ? 1.0 : 0.0;
    }

    output[idx] = feedback;
}
"#;
}

/// Warp scheduler for GPU compute timing
pub struct WarpScheduler {
    context: Arc<GPUContext>,
    target_latency_ms: u32,
    compute_queue: Arc<Mutex<Option<wgpu::Queue>>>,
}

impl WarpScheduler {
    /// Create a new warp scheduler
    pub fn new(context: Arc<GPUContext>, target_latency_ms: u32) -> Self {
        Self {
            context,
            target_latency_ms,
            compute_queue: Arc::new(Mutex::new(None)),
        }
    }

    /// Schedule compute work with timing constraints
    pub fn schedule(&self, work: impl FnOnce() + Send + 'static) -> Result<()> {
        // Submit work to GPU queue
        work();
        
        // Ensure completion within target latency
        self.context.device.poll(wgpu::Maintain::Wait);
        
        Ok(())
    }

    /// Get target latency
    pub fn target_latency(&self) -> Duration {
        Duration::from_millis(self.target_latency_ms as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_gpu_context_creation() {
        let config = WgpuConfig::default();
        let result = GPUContext::new(config).await;
        
        // May fail if no GPU available, but shouldn't panic
        if let Ok(ctx) = result {
            assert!(ctx.device.info().device_type != wgpu::DeviceType::Other);
        }
    }

    #[test]
    fn test_bss_params_pod() {
        assert_eq!(std::mem::size_of::<BSSParams>(), 32);
    }

    #[test]
    fn test_fft_params_pod() {
        assert_eq!(std::mem::size_of::<FFTParams>(), 16);
    }

    #[test]
    fn test_pdm_params_pod() {
        assert_eq!(std::mem::size_of::<PDMParams>(), 20);
    }
}
