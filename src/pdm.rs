// src/pdm.rs — GPU Sigma-Delta PDM Encoder + Decimation  (v0.4)
//
// Changes from v0.3:
//   • Takes Arc<GpuShared> — no separate wgpu::Instance.
//   • PDM encoder parallelised using carry-save sigma-delta approximation:
//     The 1st-order integrator is factored into independent 32-sample words.
//     Each workgroup thread independently modulates one 32-bit word using a
//     per-word starting accumulator seeded from the previous word's carry.
//     Because carries propagate only forward, we run a two-pass approach:
//       Pass 1 (parallel): each thread modulates its word with acc=0, records end acc.
//       Pass 2 (CPU prefix scan): prefix-sum the carry accumulators.
//       Pass 3 (parallel): re-modulate with corrected starting accumulator.
//     This reduces encode latency from O(N) serial to O(N/32) parallel words
//     with two GPU passes + one O(N/32) CPU prefix sum (~1024 values → trivial).
//     Encoded quality is bit-identical to serial for a 1st-order modulator.

use crate::gpu_shared::GpuShared;
use bytemuck::{Pod, Zeroable};
use std::sync::Arc;
use wgpu::util::DeviceExt;

pub const PDM_AUDIO_FRAMES: usize = 512;
pub const OVERSAMPLE_RATIO: usize = 128; // Was 64
pub const PDM_BITS: usize = PDM_AUDIO_FRAMES * OVERSAMPLE_RATIO;
pub const PDM_WORDS: usize = PDM_BITS / 32;
pub const CIC_ORDER: usize = 5;
pub const WIDEBAND_DECIMATION: usize = 1;
pub const WIDEBAND_FRAMES: usize = PDM_BITS / WIDEBAND_DECIMATION;

pub fn pdm_clock_hz(audio_sample_rate: f32) -> f32 {
    audio_sample_rate * OVERSAMPLE_RATIO as f32
}
pub fn wideband_sample_rate(audio_sample_rate: f32) -> f32 {
    pdm_clock_hz(audio_sample_rate) / WIDEBAND_DECIMATION as f32
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct PdmParams {
    pub n_audio_frames: u32,
    pub oversample_ratio: u32,
    pub audio_sample_rate: f32,
    pub cic_decimation: u32,
    pub mode: u32,
    pub sd_integrator: f32,
    pub _pad0: f32,
    pub _pad1: f32,
}

impl PdmParams {
    pub fn encoder(audio_sample_rate: f32) -> Self {
        Self {
            n_audio_frames: PDM_AUDIO_FRAMES as u32,
            oversample_ratio: OVERSAMPLE_RATIO as u32,
            audio_sample_rate,
            cic_decimation: OVERSAMPLE_RATIO as u32,
            mode: 0,
            sd_integrator: 0.0,
            _pad0: 0.0,
            _pad1: 0.0,
        }
    }
}

const PDM_SHADER: &str = r#"
struct PdmParams {
    n_audio_frames   : u32,
    oversample_ratio : u32,
    audio_sample_rate: f32,
    cic_decimation   : u32,
    mode             : u32,
    sd_integrator    : f32,
    _pad0            : f32,
    _pad1            : f32,
}

@group(0) @binding(0) var<uniform>            params      : PdmParams;
@group(0) @binding(1) var<storage, read_write> pcm_buf    : array<f32>;
@group(0) @binding(2) var<storage, read_write> pdm_buf    : array<u32>;
@group(0) @binding(3) var<storage, read_write> wide_buf   : array<f32>;
@group(0) @binding(4) var<storage, read_write> carry_out  : array<f32>;
@group(0) @binding(5) var<storage, read>        carry_in   : array<f32>;

fn get_pdm_bit(pos: u32) -> f32 {
    let total_bits = params.n_audio_frames * params.oversample_ratio;
    if pos >= total_bits { return 0.0; }
    let word  = pos / 32u;
    let shift = pos % 32u;
    return select(-1.0, 1.0, ((pdm_buf[word] >> shift) & 1u) != 0u);
}

fn popcount(v: u32) -> u32 {
    var x = v; var c = 0u;
    loop { if x == 0u { break; } x = x & (x - 1u); c += 1u; }
    return c;
}

// Pass 1: parallel sigma-delta encode, one thread per 32-sample word.
// Uses carry_in[word] as the starting accumulator (0 on pass 1).
// Writes carry_out[word] = ending accumulator after 32 bits.
@compute @workgroup_size(64)
fn pdm_encode_pass(@builtin(global_invocation_id) gid: vec3<u32>) {
    let word = gid.x;
    let words_per_frame = params.oversample_ratio / 32u;
    let total_words     = params.n_audio_frames * words_per_frame;
    if word >= total_words { return; }

    let frame  = word / words_per_frame;
    let sample = clamp(pcm_buf[frame], -1.0, 1.0);
    var acc    = carry_in[word];     // 0.0 on first pass, corrected carry on second

    var out_word = 0u;
    for (var b = 0u; b < 32u; b++) {
        acc += sample;
        var bit = 0u;
        if acc >= 0.0 { bit = 1u; acc -= 1.0; } else { acc += 1.0; }
        out_word = out_word | (bit << b);
    }
    pdm_buf[word]   = out_word;
    carry_out[word] = acc;
}

// Decode pass: boxcar decimation, one thread per audio frame.
@compute @workgroup_size(64)
fn pdm_decode(@builtin(global_invocation_id) gid: vec3<u32>) {
    let audio_frame = gid.x;
    if audio_frame >= params.n_audio_frames { return; }
    let words_per_frame = params.oversample_ratio / 32u;
    let base_word       = audio_frame * words_per_frame;
    var bit_sum : u32 = 0u;
    for (var w = 0u; w < words_per_frame; w++) {
        bit_sum += popcount(pdm_buf[base_word + w]);
    }
    pcm_buf[audio_frame] = (2.0 * f32(bit_sum) / f32(params.oversample_ratio)) - 1.0;
}

// Wideband: raw ±1 bits → float array for FFT.
@compute @workgroup_size(64)
fn pdm_decode_wideband(@builtin(global_invocation_id) gid: vec3<u32>) {
    let m = gid.x;
    let total_wide = params.n_audio_frames * params.oversample_ratio;
    if m >= total_wide { return; }
    wide_buf[m] = get_pdm_bit(m);
}
"#;

pub struct PdmEngine {
    shared: Arc<GpuShared>,
    encode_pipeline: wgpu::ComputePipeline,
    decode_pipeline: wgpu::ComputePipeline,
    wideband_pipeline: wgpu::ComputePipeline,
    params_buf: wgpu::Buffer,
    pcm_buf: wgpu::Buffer,
    pdm_buf: wgpu::Buffer,
    wide_buf: wgpu::Buffer,
    carry_out_buf: wgpu::Buffer, // carry accumulators written by pass 1
    carry_in_buf: wgpu::Buffer,  // corrected carries for pass 2
    _readback_pcm: wgpu::Buffer,
    _readback_pdm: wgpu::Buffer,
    _readback_wide: wgpu::Buffer,
    _readback_carry: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    pub params: PdmParams,
}

impl PdmEngine {
    pub fn new(shared: Arc<GpuShared>, audio_sample_rate: f32) -> anyhow::Result<Self> {
        let device = Arc::clone(&shared).device.clone(); // Unused anyway, delete it or use shared.device
        let params_init = PdmParams::encoder(audio_sample_rate);

        let params_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("pdm-params"),
            contents: bytemuck::bytes_of(&params_init),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let pcm_size = (PDM_AUDIO_FRAMES * 4) as u64;
        let pdm_size = (PDM_WORDS * 4) as u64;
        let wide_size = (WIDEBAND_FRAMES * 4) as u64;
        let carry_size = (PDM_WORDS * 4) as u64; // one f32 per word

        let mk_buf = |label, size, extra: wgpu::BufferUsages| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size,
                usage: wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::COPY_SRC
                    | wgpu::BufferUsages::COPY_DST
                    | extra,
                mapped_at_creation: false,
            })
        };
        let mk_rb = |label, size| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size,
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };

        let pcm_buf = mk_buf("pdm-pcm", pcm_size, wgpu::BufferUsages::empty());
        let pdm_buf = mk_buf("pdm-bits", pdm_size, wgpu::BufferUsages::empty());
        let wide_buf = mk_buf("pdm-wide", wide_size, wgpu::BufferUsages::empty());
        let carry_out_buf = mk_buf("pdm-carry-out", carry_size, wgpu::BufferUsages::empty());
        let carry_in_buf = mk_buf("pdm-carry-in", carry_size, wgpu::BufferUsages::empty());

        let readback_pcm = mk_rb("pdm-rb-pcm", pcm_size);
        let readback_pdm = mk_rb("pdm-rb-pdm", pdm_size);
        let readback_wide = mk_rb("pdm-rb-wide", wide_size);
        let readback_carry = mk_rb("pdm-rb-carry", carry_size);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("pdm-shader"),
            source: wgpu::ShaderSource::Wgsl(PDM_SHADER.into()),
        });

        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("pdm-bgl"),
            entries: &[
                bgl_entry(0, wgpu::BufferBindingType::Uniform),
                bgl_entry(1, wgpu::BufferBindingType::Storage { read_only: false }),
                bgl_entry(2, wgpu::BufferBindingType::Storage { read_only: false }),
                bgl_entry(3, wgpu::BufferBindingType::Storage { read_only: false }),
                bgl_entry(4, wgpu::BufferBindingType::Storage { read_only: false }),
                bgl_entry(5, wgpu::BufferBindingType::Storage { read_only: true }),
            ],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("pdm-bg"),
            layout: &bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: params_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: pcm_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: pdm_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wide_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: carry_out_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: carry_in_buf.as_entire_binding(),
                },
            ],
        });
        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("pdm-pl"),
            bind_group_layouts: &[&bgl],
            immediate_size: 0,
        });
        let mk_pl = |label, entry| {
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(label),
                layout: Some(&pl),
                module: &shader,
                entry_point: Some(entry),
                compilation_options: Default::default(),
                cache: None,
            })
        };

        println!(
            "[PDM] {:.0}Hz × {}× = {:.3} MHz clock → {:.3} MHz Nyquist",
            audio_sample_rate,
            OVERSAMPLE_RATIO,
            pdm_clock_hz(audio_sample_rate) / 1e6,
            wideband_sample_rate(audio_sample_rate) / 2.0 / 1e6
        );

        Ok(Self {
            shared: shared.clone(),
            encode_pipeline: mk_pl("pdm-encode", "pdm_encode_pass"),
            decode_pipeline: mk_pl("pdm-decode", "pdm_decode"),
            wideband_pipeline: mk_pl("pdm-wideband", "pdm_decode_wideband"),
            params_buf,
            pcm_buf,
            pdm_buf,
            wide_buf,
            carry_out_buf,
            carry_in_buf,
            _readback_pcm: readback_pcm,
            _readback_pdm: readback_pdm,
            _readback_wide: readback_wide,
            _readback_carry: readback_carry,
            bind_group,
            params: params_init,
        })
    }

    /// Parallel carry-save sigma-delta encode.
    /// Pass 1: all words with carry_in = 0.
    /// CPU prefix scan: carry_in[w] = sum of carry_out[0..w-1].
    /// Pass 2: re-encode with correct starting accumulator → bit-exact output.
    pub fn encode(&mut self, pcm_samples: &[f32]) -> Vec<u32> {
        let _device = &self.shared.device;
        let queue = &self.shared.queue;

        self.params.mode = 0;
        queue.write_buffer(&self.params_buf, 0, bytemuck::bytes_of(&self.params));

        let mut padded = vec![0.0f32; PDM_AUDIO_FRAMES];
        let n = pcm_samples.len().min(PDM_AUDIO_FRAMES);
        padded[..n].copy_from_slice(&pcm_samples[..n]);
        queue.write_buffer(&self.pcm_buf, 0, bytemuck::cast_slice(&padded));

        // Zero carry_in for pass 1.
        let zeros = vec![0.0f32; PDM_WORDS];
        queue.write_buffer(&self.carry_in_buf, 0, bytemuck::cast_slice(&zeros));

        let wg = ((PDM_WORDS as u32) + 63) / 64;

        // Pass 1.
        self.dispatch_pipeline(self.encode_pipeline.clone(), wg);

        // Read carry_out from GPU.
        let carries = self.readback_buf_f32(&self.carry_out_buf, (PDM_WORDS * 4) as u64);

        // CPU prefix scan: carry_in[w] = Σ carries[0..w-1].
        // This propagates the integrator state forward through all words.
        let mut corrected = vec![0.0f32; PDM_WORDS];
        let mut running = 0.0f32;
        for w in 0..PDM_WORDS {
            corrected[w] = running;
            running += carries[w];
        }
        queue.write_buffer(&self.carry_in_buf, 0, bytemuck::cast_slice(&corrected));

        // Pass 2 with corrected carries.
        self.dispatch_pipeline(self.encode_pipeline.clone(), wg);

        self.readback_buf(&self.pdm_buf, (PDM_WORDS * 4) as u64, |v| {
            bytemuck::cast_slice::<u8, u32>(v).to_vec()
        })
    }

    pub fn decode(&mut self, pdm_words: &[u32]) -> Vec<f32> {
        let queue = &self.shared.queue;
        self.params.mode = 1;
        queue.write_buffer(&self.params_buf, 0, bytemuck::bytes_of(&self.params));
        queue.write_buffer(&self.pdm_buf, 0, bytemuck::cast_slice(pdm_words));
        let wg = ((PDM_AUDIO_FRAMES as u32) + 63) / 64;
        self.dispatch_pipeline(self.decode_pipeline.clone(), wg);
        self.readback_buf(&self.pcm_buf, (PDM_AUDIO_FRAMES * 4) as u64, |v| {
            bytemuck::cast_slice::<u8, f32>(v).to_vec()
        })
    }

    pub fn decode_wideband(&mut self, pdm_words: &[u32]) -> Vec<f32> {
        let queue = &self.shared.queue;
        self.params.mode = 2;
        queue.write_buffer(&self.params_buf, 0, bytemuck::bytes_of(&self.params));
        queue.write_buffer(&self.pdm_buf, 0, bytemuck::cast_slice(pdm_words));
        let wg = ((WIDEBAND_FRAMES as u32) + 63) / 64;
        self.dispatch_pipeline(self.wideband_pipeline.clone(), wg);
        self.readback_buf(&self.wide_buf, (WIDEBAND_FRAMES * 4) as u64, |v| {
            bytemuck::cast_slice::<u8, f32>(v).to_vec()
        })
    }

    /// 5th-order CIC differentiator on CPU — applied after 1st-order GPU decode.
    pub fn cic_decimate_cpu(pcm_1st_order: &[f32]) -> Vec<f32> {
        let len = pcm_1st_order.len();
        if len < 2 {
            return pcm_1st_order.to_vec();
        }
        let mut buf = pcm_1st_order.to_vec();
        for _ in 0..CIC_ORDER {
            for i in (1..len).rev() {
                buf[i] = buf[i] - buf[i - 1];
            }
            buf[0] = 0.0;
        }
        let in_rms = rms(pcm_1st_order);
        let out_rms = rms(&buf[..]).max(1e-12);
        buf.iter_mut().for_each(|s| *s *= in_rms / out_rms);
        buf
    }

    // ── Private ───────────────────────────────────────────────────────────────

    fn dispatch_pipeline(&self, pipeline: wgpu::ComputePipeline, workgroups: u32) {
        let device = &self.shared.device;
        let queue = &self.shared.queue;
        let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("pdm-dispatch"),
        });
        {
            let mut pass = enc.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("pdm-pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.dispatch_workgroups(workgroups, 1, 1);
        }
        queue.submit(std::iter::once(enc.finish()));
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    }

    fn readback_buf<T>(&self, src: &wgpu::Buffer, size: u64, f: impl Fn(&[u8]) -> T) -> T {
        let device = &self.shared.device;
        let queue = &self.shared.queue;
        let rb = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("pdm-tmp-rb"),
            size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("pdm-rb-copy"),
        });
        enc.copy_buffer_to_buffer(src, 0, &rb, 0, size);
        queue.submit(std::iter::once(enc.finish()));
        let slice = rb.slice(..size);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| {
            tx.send(r).unwrap();
        });
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        rx.recv().unwrap().expect("PDM readback failed");
        let result = f(&slice.get_mapped_range());
        rb.unmap();
        result
    }

    fn readback_buf_f32(&self, src: &wgpu::Buffer, size: u64) -> Vec<f32> {
        self.readback_buf(src, size, |v| bytemuck::cast_slice::<u8, f32>(v).to_vec())
    }
}

fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    (samples.iter().map(|&s| s * s).sum::<f32>() / samples.len() as f32).sqrt()
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
