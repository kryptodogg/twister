// src/waterfall.rs — GPU Waterfall Display Engine  (v0.4)
//
// Changes from v0.3:
//   • Takes Arc<GpuShared> — no separate wgpu::Instance.
//   • push_row() signature unchanged; called with magnitudes sliced from V-buffer
//     by the dispatch thread.
//   • WGSL float literals corrected (no `f32` suffix — WGSL doesn't support it).
//   • min_freq_hz default 1.0 Hz (no 20 Hz floor).

use std::sync::Arc;
use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;
use crate::gpu_shared::GpuShared;

pub const WATERFALL_BINS:  usize = 512;
pub const WATERFALL_ROWS:  usize = 128;
pub const WATERFALL_CELLS: usize = WATERFALL_BINS * WATERFALL_ROWS;
pub const SPECTRUM_BINS:   usize = 256;

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct WaterfallParams {
    pub n_bins:      u32,
    pub n_rows:      u32,
    pub current_row: u32,
    pub peak_mag:    f32,
    pub floor_db:    f32,
    pub range_db:    f32,
    pub pdm_mode:    u32,
    pub sample_rate: f32,
    pub raw_bins:    u32,
    pub min_freq_hz: f32,
    pub max_freq_hz: f32,
    pub _pad0:       u32,
}

const WATERFALL_SHADER: &str = r#"
struct WaterfallParams {
    n_bins      : u32,
    n_rows      : u32,
    current_row : u32,
    peak_mag    : f32,
    floor_db    : f32,
    range_db    : f32,
    pdm_mode    : u32,
    sample_rate : f32,
    raw_bins    : u32,
    min_freq_hz : f32,
    max_freq_hz : f32,
    _pad0       : u32,
}

@group(0) @binding(0) var<uniform>             params       : WaterfallParams;
@group(0) @binding(1) var<storage, read>        raw_mag      : array<f32>;
@group(0) @binding(2) var<storage, read_write>  mag_row      : array<f32>;
@group(0) @binding(3) var<storage, read_write>  history      : array<f32>;
@group(0) @binding(4) var<storage, read_write>  rgba_out     : array<u32>;
@group(0) @binding(5) var<storage, read_write>  spectrum_out : array<f32>;

fn colormap(t: f32) -> vec4<f32> {
    let v = clamp(t, 0.0, 1.0);
    if v < 0.143 { return mix(vec4<f32>(0.0,0.0,0.0,1.0),    vec4<f32>(0.07,0.0,0.20,1.0),  v/0.143); }
    if v < 0.286 { return mix(vec4<f32>(0.07,0.0,0.20,1.0),  vec4<f32>(0.01,0.19,0.50,1.0), (v-0.143)/0.143); }
    if v < 0.429 { return mix(vec4<f32>(0.01,0.19,0.50,1.0), vec4<f32>(0.0,0.50,0.70,1.0),  (v-0.286)/0.143); }
    if v < 0.571 { return mix(vec4<f32>(0.0,0.50,0.70,1.0),  vec4<f32>(0.0,0.73,0.55,1.0),  (v-0.429)/0.143); }
    if v < 0.714 { return mix(vec4<f32>(0.0,0.73,0.55,1.0),  vec4<f32>(0.45,0.82,0.30,1.0), (v-0.571)/0.143); }
    if v < 0.857 { return mix(vec4<f32>(0.45,0.82,0.30,1.0), vec4<f32>(0.90,0.86,0.10,1.0), (v-0.714)/0.143); }
    return mix(vec4<f32>(0.90,0.86,0.10,1.0), vec4<f32>(1.0,1.0,0.9,1.0), (v-0.857)/0.143);
}

fn pack_rgba(c: vec4<f32>) -> u32 {
    let r = u32(clamp(c.x*255.0, 0.0, 255.0));
    let g = u32(clamp(c.y*255.0, 0.0, 255.0));
    let b = u32(clamp(c.z*255.0, 0.0, 255.0));
    let a = u32(clamp(c.w*255.0, 0.0, 255.0));
    return r | (g << 8u) | (b << 16u) | (a << 24u);
}

fn bin_to_raw_idx(bin: f32, total_bins: f32) -> u32 {
    let t     = bin / total_bins;
    let freq  = params.min_freq_hz * pow(params.max_freq_hz / params.min_freq_hz, t);
    let raw_f = (freq / params.max_freq_hz) * f32(params.raw_bins);
    return u32(clamp(raw_f, 0.0, f32(params.raw_bins) - 1.0));
}

@compute @workgroup_size(64)
fn downsample_raw(@builtin(global_invocation_id) gid: vec3<u32>) {
    let bin = gid.x;
    if bin >= params.n_bins { return; }

    let start    = bin_to_raw_idx(f32(bin),       f32(params.n_bins));
    let end      = max(start + 1u, bin_to_raw_idx(f32(bin + 1u), f32(params.n_bins)));
    var peak_val = 0.0;
    for (var i = start; i < end; i++) { peak_val = max(peak_val, raw_mag[i]); }
    mag_row[bin] = peak_val;

    if bin < 256u {
        let s_start = bin_to_raw_idx(f32(bin),       256.0);
        let s_end   = max(s_start + 1u, bin_to_raw_idx(f32(bin + 1u), 256.0));
        var s_peak  = 0.0;
        for (var i = s_start; i < s_end; i++) { s_peak = max(s_peak, raw_mag[i]); }
        let peak_ref = max(params.peak_mag, 1e-6);
        let db = 20.0 * log(s_peak / peak_ref + 1e-9) / log(10.0);
        spectrum_out[bin] = clamp((db - params.floor_db) / params.range_db, 0.0, 1.0);
    }
}

@compute @workgroup_size(64)
fn scroll_and_insert(@builtin(global_invocation_id) gid: vec3<u32>) {
    let bin = gid.x;
    if bin >= params.n_bins { return; }
    for (var row = params.n_rows - 1u; row > 0u; row--) {
        history[row * params.n_bins + bin] = history[(row - 1u) * params.n_bins + bin];
    }
    history[bin] = mag_row[bin];
}

@compute @workgroup_size(64)
fn colormap_all(@builtin(global_invocation_id) gid: vec3<u32>) {
    let cell = gid.x;
    if cell >= params.n_bins * params.n_rows { return; }
    let mag      = history[cell];
    let peak_ref = max(params.peak_mag, 1e-6);
    let db       = 20.0 * log(mag / peak_ref + 1e-9) / log(10.0);
    let t        = clamp((db - params.floor_db) / params.range_db, 0.0, 1.0);
    rgba_out[cell] = pack_rgba(colormap(t));
}
"#;

pub struct WaterfallEngine {
    shared:              Arc<GpuShared>,
    downsample_pipeline: wgpu::ComputePipeline,
    scroll_pipeline:     wgpu::ComputePipeline,
    color_pipeline:      wgpu::ComputePipeline,
    params_buf:          wgpu::Buffer,
    raw_mag_buf:         wgpu::Buffer,
    _mag_row_buf:        wgpu::Buffer,
    _history_buf:        wgpu::Buffer,
    rgba_buf:            wgpu::Buffer,
    spectrum_buf:        wgpu::Buffer,
    readback_rgba:       wgpu::Buffer,
    readback_spectrum:   wgpu::Buffer,
    bind_group:          wgpu::BindGroup,
    pub params:          WaterfallParams,
    auto_peak:           f32,
}

impl WaterfallEngine {
    pub fn new(shared: Arc<GpuShared>, sample_rate: f32, pdm_mode: bool) -> anyhow::Result<Self> {
        let device = &shared.device;

        let params_init = WaterfallParams {
            n_bins: WATERFALL_BINS as u32, n_rows: WATERFALL_ROWS as u32,
            current_row: 0, peak_mag: 1.0,
            floor_db: -80.0, range_db: 70.0,
            pdm_mode: if pdm_mode { 1 } else { 0 },
            sample_rate, raw_bins: 0,
            min_freq_hz: 1.0, max_freq_hz: sample_rate / 2.0, _pad0: 0,
        };

        let params_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("wf-params"), contents: bytemuck::bytes_of(&params_init),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let max_raw_size  = (32_768 * 4) as u64;
        let mag_row_size  = (WATERFALL_BINS  * 4) as u64;
        let history_size  = (WATERFALL_CELLS * 4) as u64;
        let rgba_size     = (WATERFALL_CELLS * 4) as u64;
        let spectrum_size = (SPECTRUM_BINS   * 4) as u64;

        let mk_storage = |label: &'static str, size: u64, extra: wgpu::BufferUsages| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label), size,
                usage: wgpu::BufferUsages::STORAGE | extra,
                mapped_at_creation: false,
            })
        };
        let mk_readback = |label: &'static str, size: u64| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label), size,
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };

        let raw_mag_buf     = mk_storage("wf-raw-mag",  max_raw_size,  wgpu::BufferUsages::COPY_DST);
        let mag_row_buf     = mk_storage("wf-mag-row",  mag_row_size,  wgpu::BufferUsages::empty());
        let history_buf     = mk_storage("wf-history",  history_size,  wgpu::BufferUsages::empty());
        let rgba_buf        = mk_storage("wf-rgba",     rgba_size,     wgpu::BufferUsages::COPY_SRC);
        let spectrum_buf    = mk_storage("wf-spectrum", spectrum_size, wgpu::BufferUsages::COPY_SRC);
        let readback_rgba   = mk_readback("wf-rb-rgba",    rgba_size);
        let readback_spectrum = mk_readback("wf-rb-spectrum", spectrum_size);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("waterfall-shader"),
            source: wgpu::ShaderSource::Wgsl(WATERFALL_SHADER.into()),
        });

        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("wf-bgl"),
            entries: &[
                bgl_entry(0, wgpu::BufferBindingType::Uniform),
                bgl_entry(1, wgpu::BufferBindingType::Storage { read_only: true }),
                bgl_entry(2, wgpu::BufferBindingType::Storage { read_only: false }),
                bgl_entry(3, wgpu::BufferBindingType::Storage { read_only: false }),
                bgl_entry(4, wgpu::BufferBindingType::Storage { read_only: false }),
                bgl_entry(5, wgpu::BufferBindingType::Storage { read_only: false }),
            ],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("wf-bg"), layout: &bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: params_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: raw_mag_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: mag_row_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 3, resource: history_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 4, resource: rgba_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 5, resource: spectrum_buf.as_entire_binding() },
            ],
        });
        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("wf-pl"), bind_group_layouts: &[&bgl], immediate_size: 0,
        });

        let mk_pipeline = |label, entry| device.create_compute_pipeline(
            &wgpu::ComputePipelineDescriptor {
                label: Some(label), layout: Some(&pl), module: &shader,
                entry_point: Some(entry),
                compilation_options: Default::default(), cache: None,
            }
        );

        Ok(Self {
            shared: shared.clone(),
            downsample_pipeline: mk_pipeline("wf-downsample", "downsample_raw"),
            scroll_pipeline:     mk_pipeline("wf-scroll",     "scroll_and_insert"),
            color_pipeline:      mk_pipeline("wf-colormap",   "colormap_all"),
            params_buf, raw_mag_buf,
            _mag_row_buf: mag_row_buf, _history_buf: history_buf,
            rgba_buf, spectrum_buf, readback_rgba, readback_spectrum,
            bind_group, params: params_init, auto_peak: 1.0,
        })
    }

    pub fn push_row(&mut self, raw_magnitudes: &[f32], min_freq: f32, max_freq: f32) -> (Vec<u32>, Vec<f32>) {
        let device = &self.shared.device;
        let queue  = &self.shared.queue;

        let n = raw_magnitudes.len().min(32_768);
        self.params.raw_bins    = n as u32;
        self.params.min_freq_hz = min_freq.max(1.0);
        self.params.max_freq_hz = max_freq;

        let row_peak = raw_magnitudes[..n].iter().cloned().fold(0.0f32, f32::max);
        self.auto_peak = (self.auto_peak * 0.98 + row_peak * 0.02).max(1e-6);
        self.params.peak_mag = self.auto_peak;

        queue.write_buffer(&self.params_buf, 0, bytemuck::bytes_of(&self.params));
        queue.write_buffer(&self.raw_mag_buf, 0, bytemuck::cast_slice(&raw_magnitudes[..n]));

        let mut encoder = device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor { label: Some("wf-enc") });
        {
            let mut pass = encoder.begin_compute_pass(
                &wgpu::ComputePassDescriptor { label: Some("wf-pass"), timestamp_writes: None });
            let wg_bins  = ((WATERFALL_BINS  as u32) + 63) / 64;
            let wg_cells = ((WATERFALL_CELLS as u32) + 63) / 64;
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.set_pipeline(&self.downsample_pipeline);
            pass.dispatch_workgroups(wg_bins, 1, 1);
            pass.set_pipeline(&self.scroll_pipeline);
            pass.dispatch_workgroups(wg_bins, 1, 1);
            pass.set_pipeline(&self.color_pipeline);
            pass.dispatch_workgroups(wg_cells, 1, 1);
        }

        let rgba_bytes     = (WATERFALL_CELLS * 4) as u64;
        let spectrum_bytes = (SPECTRUM_BINS   * 4) as u64;
        encoder.copy_buffer_to_buffer(&self.rgba_buf,     0, &self.readback_rgba,     0, rgba_bytes);
        encoder.copy_buffer_to_buffer(&self.spectrum_buf, 0, &self.readback_spectrum, 0, spectrum_bytes);
        queue.submit(std::iter::once(encoder.finish()));

        let sl_rgba = self.readback_rgba.slice(..);
        let sl_spec = self.readback_spectrum.slice(..);
        let (t1, r1) = std::sync::mpsc::channel();
        let (t2, r2) = std::sync::mpsc::channel();
        sl_rgba.map_async(wgpu::MapMode::Read, move |r| { t1.send(r).unwrap(); });
        sl_spec.map_async(wgpu::MapMode::Read, move |r| { t2.send(r).unwrap(); });
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        r1.recv().unwrap().expect("WF RGBA readback failed");
        r2.recv().unwrap().expect("WF spectrum readback failed");

        let rgba_data = bytemuck::cast_slice::<u8, u32>(&sl_rgba.get_mapped_range()).to_vec();
        let spec_data = bytemuck::cast_slice::<u8, f32>(&sl_spec.get_mapped_range()).to_vec();
        self.readback_rgba.unmap();
        self.readback_spectrum.unmap();
        (rgba_data, spec_data)
    }

    pub fn set_pdm_mode(&mut self, pdm: bool) {
        self.params.pdm_mode = if pdm { 1 } else { 0 };
    }
}

fn bgl_entry(binding: u32, ty: wgpu::BufferBindingType) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding, visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer { ty, has_dynamic_offset: false, min_binding_size: None },
        count: None,
    }
}
