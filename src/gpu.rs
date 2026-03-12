// src/gpu.rs — GPU Synthesis and Signal Processing Engine
//
// Orchestrates:
//   1. STFT + Coherence compute passes on the GPU (SignalProcessorGpu).
//   2. 1 Hz - 96 MHz continuous wave synthesis (GpuContext).
//
// All engines share one physical Device / Queue via Arc<GpuShared>.

use crate::gpu_shared::GpuShared;
use crate::vbuffer::{GpuVBuffer, IqVBuffer, V_FREQ_BINS, VBufferPushConst};
use bytemuck::{Pod, Zeroable};
use std::sync::Arc;
use wgpu::util::DeviceExt;

pub const SYNTH_FRAMES: usize = 512;
pub const MAX_CHANNELS: usize = 8;
pub const MAX_DENIAL_TARGETS: usize = 16;

// ── GPU Synthesis Engine (GpuContext) ───────────────────────────────────────────

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct GpuDenialTarget {
    pub freq_hz: f32,
    pub gain: f32,
    pub phase: f32,
    pub is_active: f32,
}

impl GpuDenialTarget {
    pub const fn inactive() -> Self {
        Self {
            freq_hz: 0.0,
            gain: 0.0,
            phase: 0.0,
            is_active: 0.0,
        }
    }
}

/// Uniform block — 16-byte aligned throughout.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct SynthParams {
    pub sample_rate: f32,
    pub master_gain: f32,
    pub mode: u32,
    pub n_channels: u32,
    // vec4 ──
    pub n_frames: u32,
    pub sweep_freq: f32,
    pub lcg_state: u32,
    pub sweep_dir: f32,
    // vec4 ──
    pub waveshape: u32,
    pub waveshape_drive: f32,
    pub polarization: f32,
    pub beam_half_width: f32,
    // targets: 16 × 4 f32 = 256 bytes
    pub targets: [GpuDenialTarget; MAX_DENIAL_TARGETS],
}

impl SynthParams {
    pub fn new(sample_rate: f32, n_channels: u32) -> Self {
        Self {
            sample_rate,
            master_gain: 0.5,
            mode: 0,
            n_channels,
            n_frames: SYNTH_FRAMES as u32,
            sweep_freq: 1.0,
            lcg_state: 0xDEAD_BEEF,
            sweep_dir: 1.0,
            waveshape: 0,
            waveshape_drive: 0.5,
            polarization: 0.0,
            beam_half_width: std::f32::consts::FRAC_PI_2,
            targets: [GpuDenialTarget::inactive(); MAX_DENIAL_TARGETS],
        }
    }

    pub fn set_targets(&mut self, freqs: &[(f32, f32)]) {
        for t in self.targets.iter_mut() {
            *t = GpuDenialTarget::inactive();
        }
        for (i, &(freq_hz, gain)) in freqs.iter().take(MAX_DENIAL_TARGETS).enumerate() {
            self.targets[i] = GpuDenialTarget {
                freq_hz,
                gain: gain.clamp(0.0, 1.0),
                phase: self.targets[i].phase,
                is_active: 1.0,
            };
        }
    }

    pub fn advance_phase(&mut self, n_frames: usize) {
        let tau = std::f32::consts::TAU;
        for t in self.targets.iter_mut() {
            if t.is_active > 0.5 {
                t.phase = (t.phase + n_frames as f32 * tau * t.freq_hz / self.sample_rate) % tau;
            }
        }
        // Park-Miller LCG for noise mode — intentionally integer arithmetic.
        for _ in 0..n_frames {
            self.lcg_state = self
                .lcg_state
                .wrapping_mul(1_664_525)
                .wrapping_add(1_013_904_223);
        }
        let sweep_inc = 20.0 * n_frames as f32 / self.sample_rate;
        self.sweep_freq += self.sweep_dir * sweep_inc;
        let max_freq = 12_288_000.0;
        if self.sweep_freq >= max_freq || self.sweep_freq <= 1.0 {
            self.sweep_dir = -self.sweep_dir;
            self.sweep_freq = self.sweep_freq.clamp(1.0, max_freq);
        }
    }
}

// ── WGSL synthesis shader ─────────────────────────────────────────────────────

const SYNTHESIS_SHADER: &str = r#"
struct Target { freq_hz: f32, gain: f32, phase: f32, is_active: f32 }

struct SynthParams {
    sample_rate     : f32,
    master_gain     : f32,
    mode            : u32,
    n_channels      : u32,
    n_frames        : u32,
    sweep_freq      : f32,
    lcg_state       : u32,
    sweep_dir       : f32,
    waveshape       : u32,
    waveshape_drive : f32,
    polarization    : f32,
    beam_half_width : f32,
    targets         : array<Target, 16>,
}

@group(0) @binding(0) var<uniform>            params : SynthParams;
@group(0) @binding(1) var<storage, read_write> output : array<f32>;

const TAU : f32 = 6.283185307179586;
const PI  : f32 = 3.141592653589793;

fn waveshape(ph: f32, shape: u32, drive: f32) -> f32 {
    let s = sin(ph);
    switch shape {
        case 0u: { return s; }
        case 1u: { return mix(s, select(-1.0, 1.0, s >= 0.0), drive); }
        case 2u: { return mix(s, 1.0 - 4.0 * abs(ph / TAU - 0.5), drive); }
        case 3u: { return mix(s, 2.0 * (ph / TAU) - 1.0, drive); }
        case 4u: {
            let k = mix(1.0, 8.0, drive);
            return tanh(k * s) / max(tanh(k), 0.001);
        }
        default: { return s; }
    }
}

fn channel_azimuth(ch: u32) -> f32 {
    switch ch {
        case 0u: { return -0.5236; } case 1u: { return  0.5236; }
        case 2u: { return  0.0;    } case 3u: { return  0.0;    }
        case 4u: { return -2.3562; } case 5u: { return  2.3562; }
        case 6u: { return -1.5708; } case 7u: { return  1.5708; }
        default: { return 0.0; }
    }
}

fn beam_gain(ch: u32, polarization: f32, half_width: f32) -> f32 {
    // Omni mode: half_width >= PI means no beam steering, full gain on all channels
    if half_width >= PI { return 1.0; }
    var diff = channel_azimuth(ch) - polarization;
    diff = diff - TAU * round(diff / TAU);
    let norm = abs(diff) / half_width;
    if norm >= 1.0 { return 0.0; }
    let w = cos(norm * PI * 0.5);
    return w * w;
}

@compute @workgroup_size(64)
fn synthesize(@builtin(global_invocation_id) gid: vec3<u32>) {
    let frame_idx = gid.x;
    if frame_idx >= params.n_frames { return; }
    let sr   = params.sample_rate;
    let gain = params.master_gain;
    var mono : f32 = 0.0;

    switch params.mode {
        case 0u: { mono = 0.0; }
        case 1u: {
            for (var i = 0u; i < 16u; i++) {
                let t = params.targets[i];
                if t.is_active < 0.5 { continue; }
                let ph = (t.phase + f32(frame_idx) * TAU * t.freq_hz / sr) % TAU;
                mono -= waveshape(ph, params.waveshape, params.waveshape_drive) * t.gain;
            }
            mono *= gain;
        }
        case 2u: {
            var lcg = params.lcg_state ^ (frame_idx * 1664525u + 1013904223u);
            lcg = lcg * 1664525u + 1013904223u;
            mono = f32(bitcast<i32>(lcg)) / f32(0x7FFFFFFF) * gain * 0.3;
        }
        case 3u: {
            let t  = params.targets[0];
            let ph = (t.phase + f32(frame_idx) * TAU * t.freq_hz / sr) % TAU;
            mono   = waveshape(ph, params.waveshape, params.waveshape_drive) * gain;
        }
        case 4u: {
            let ph = (params.targets[0].phase + f32(frame_idx) * TAU * params.sweep_freq / sr) % TAU;
            mono   = waveshape(ph, params.waveshape, params.waveshape_drive) * gain;
        }
        case 5u: {
            // ANC counter-phase: synthesize the same as ANTI (mode 1) but the
            // Rust side will add the LMS-filtered ANC correction on top.
            // This ensures the GPU produces a carrier signal even in ANC mode.
            for (var i = 0u; i < 16u; i++) {
                let t = params.targets[i];
                if t.is_active < 0.5 { continue; }
                let ph = (t.phase + f32(frame_idx) * TAU * t.freq_hz / sr) % TAU;
                mono -= waveshape(ph, params.waveshape, params.waveshape_drive) * t.gain;
            }
            mono *= gain;
        }
        default: { mono = 0.0; }
    }

    let base = frame_idx * params.n_channels;
    for (var ch = 0u; ch < params.n_channels; ch++) {
        output[base + ch] = mono * beam_gain(ch, params.polarization, params.beam_half_width);
    }
}
"#;

// ── GpuContext ─────────────────────────────────────────────────────────────────

pub struct GpuContext {
    shared: Arc<GpuShared>,
    pipeline: wgpu::ComputePipeline,
    params_buffer: wgpu::Buffer,
    output_buffer: wgpu::Buffer,
    readback_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    pub params: SynthParams,
}

impl GpuContext {
    pub fn new(shared: Arc<GpuShared>, sample_rate: f32, n_channels: u32) -> anyhow::Result<Self> {
        let device = &shared.device;

        let params_init = SynthParams::new(sample_rate, n_channels);
        let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("synth-params"),
            contents: bytemuck::bytes_of(&params_init),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let output_size = (SYNTH_FRAMES * MAX_CHANNELS * 4) as u64;
        let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("synth-output"),
            size: output_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let readback_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("synth-readback"),
            size: output_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("synthesis-shader"),
            source: wgpu::ShaderSource::Wgsl(SYNTHESIS_SHADER.into()),
        });

        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("synth-bgl"),
            entries: &[
                bgl_entry(0, wgpu::BufferBindingType::Uniform),
                bgl_entry(1, wgpu::BufferBindingType::Storage { read_only: false }),
            ],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("synth-bg"),
            layout: &bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: output_buffer.as_entire_binding(),
                },
            ],
        });
        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("synth-pl"),
            bind_group_layouts: &[&bgl],
            immediate_size: 0,
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("synthesis-pipeline"),
            layout: Some(&pl),
            module: &shader,
            entry_point: Some("synthesize"),
            compilation_options: Default::default(),
            cache: None,
        });

        let max_tx_mhz = 12.288 / 2.0;
        println!(
            "[GPU] Synthesis ready. TX limit reported as: {:.3} MHz (Super-Nyquist aliasing allowed)",
            max_tx_mhz
        );

        Ok(Self {
            shared,
            pipeline,
            params_buffer,
            output_buffer,
            readback_buffer,
            bind_group,
            params: params_init,
        })
    }

    pub fn dispatch_synthesis(&mut self) -> Vec<f32> {
        let device = &self.shared.device;
        let queue = &self.shared.queue;

        queue.write_buffer(&self.params_buffer, 0, bytemuck::bytes_of(&self.params));

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("synth-enc"),
        });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("synth-pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.dispatch_workgroups(((SYNTH_FRAMES as u32) + 63) / 64, 1, 1);
        }
        let sz = (SYNTH_FRAMES * MAX_CHANNELS * 4) as u64;
        encoder.copy_buffer_to_buffer(&self.output_buffer, 0, &self.readback_buffer, 0, sz);
        queue.submit(std::iter::once(encoder.finish()));

        let slice = self.readback_buffer.slice(..sz);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| {
            tx.send(r).unwrap();
        });
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        rx.recv().unwrap().expect("Synth readback failed");

        let n_ch = self.params.n_channels as usize;
        let data = {
            let view = slice.get_mapped_range();
            bytemuck::cast_slice::<u8, f32>(&view)[..SYNTH_FRAMES * n_ch].to_vec()
        };
        self.readback_buffer.unmap();
        self.params.advance_phase(SYNTH_FRAMES);
        data
    }
}

fn bgl_entry(binding: u32, ty: wgpu::BufferBindingType) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

// ── Signal Processor Engine (SignalProcessorGpu) ────────────────────────────────

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
            source: wgpu::ShaderSource::Wgsl(
                include_str!("visualization/shaders/stft_iq.wgsl").into(),
            ),
        });

        let coherence_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("coherence.wgsl"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("visualization/shaders/coherence.wgsl").into(),
            ),
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

            cpass.dispatch_workgroups(V_FREQ_BINS as u32, 1, 1);
        }

        queue.submit(std::iter::once(encoder.finish()));
        gpu_vbuffer.meta.version += 1;
    }
}
pub mod wavelet_compute;
pub use wavelet_compute::WaveletComputePipeline;
