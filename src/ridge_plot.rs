// src/ridge_plot.rs — GPU-Accelerated 3D Ridge Plot Waterfall
//
// Joy Division "Unknown Pleasures" style stacked waveform visualization.
// Uses a wgpu compute shader to render the ridge plot directly on GPU.
// Each pixel is evaluated in parallel — the shader iterates through spectrum
// history rows back-to-front, drawing waveform lines with glow and occlusion.
//
// Data flow:
//   CPU uploads spectrum_history (flat f32 array) → GPU computes RGBA pixels →
//   readback → SharedPixelBuffer<Rgba8Pixel> → Slint Image.

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

/// Number of spectrum history rows to display.
pub const RIDGE_ROWS: usize = 48;

/// Number of spectrum bins per row (matches SPECTRUM_BINS from waterfall.rs).
pub const RIDGE_BINS: usize = 256;

/// Output image dimensions — FemtoVG bilinear-scales from here.
pub const RIDGE_W: u32 = 640;
pub const RIDGE_H: u32 = 420;

const RIDGE_PIXELS: usize = (RIDGE_W * RIDGE_H) as usize;
const HISTORY_FLOATS: usize = RIDGE_ROWS * RIDGE_BINS;

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct RidgeParams {
    pub width: u32,
    pub height: u32,
    pub n_rows: u32,
    pub n_bins: u32,
    pub row_step: f32,
    pub amp_scale: f32,
    pub margin_x: f32,
    pub margin_bottom: f32,
    // Background color
    pub bg_r: f32,
    pub bg_g: f32,
    pub bg_b: f32,
    pub perspective_shrink: f32,
}

const RIDGE_SHADER: &str = r#"
struct RidgeParams {
    width            : u32,
    height           : u32,
    n_rows           : u32,
    n_bins           : u32,
    row_step         : f32,
    amp_scale        : f32,
    margin_x         : f32,
    margin_bottom    : f32,
    bg_r             : f32,
    bg_g             : f32,
    bg_b             : f32,
    perspective_shrink : f32,
}

@group(0) @binding(0) var<uniform>             params  : RidgeParams;
@group(0) @binding(1) var<storage, read>       history : array<f32>;
@group(0) @binding(2) var<storage, read_write> rgba_out: array<u32>;

fn pack_rgba(c: vec4<f32>) -> u32 {
    let r = u32(clamp(c.x * 255.0, 0.0, 255.0));
    let g = u32(clamp(c.y * 255.0, 0.0, 255.0));
    let b = u32(clamp(c.z * 255.0, 0.0, 255.0));
    let a = u32(clamp(c.w * 255.0, 0.0, 255.0));
    return r | (g << 8u) | (b << 16u) | (a << 24u);
}

fn lerp3(a: vec3<f32>, b: vec3<f32>, t: f32) -> vec3<f32> {
    return a * (1.0 - t) + b * t;
}

// Sample spectrum history with linear interpolation between bins.
fn sample_spectrum(row: u32, bin_f: f32) -> f32 {
    let bin  = u32(bin_f);
    let next = min(bin + 1u, params.n_bins - 1u);
    let frac = bin_f - f32(bin);
    let a = history[row * params.n_bins + bin];
    let b = history[row * params.n_bins + next];
    return a * (1.0 - frac) + b * frac;
}

@compute @workgroup_size(8, 8)
fn ridge_render(@builtin(global_invocation_id) gid: vec3<u32>) {
    let px = gid.x;
    let py = gid.y;
    if px >= params.width || py >= params.height { return; }

    let w  = f32(params.width);
    let h  = f32(params.height);
    let bg = vec3(params.bg_r, params.bg_g, params.bg_b);

    let usable_w = w - 2.0 * params.margin_x;
    let front_y  = h - params.margin_bottom;
    let fy       = f32(py);
    let fx       = f32(px);

    var color = bg;

    // Draw rows back-to-front.
    // Row index 0 = oldest (back), n_rows-1 = newest (front).
    let nr = params.n_rows;
    if nr == 0u {
        rgba_out[py * params.width + px] = pack_rgba(vec4(bg, 1.0));
        return;
    }

    for (var row_idx = 0u; row_idx < nr; row_idx++) {
        // Depth: 0.0 = front (newest), 1.0 = back (oldest).
        let depth     = f32(nr - 1u - row_idx);
        let depth_max = max(f32(nr - 1u), 1.0);
        let depth_frac = depth / depth_max;

        // Y baseline — rear rows float higher.
        let y_base = front_y - depth * params.row_step;

        // Perspective narrowing for rear rows.
        let perspective = 1.0 - depth_frac * params.perspective_shrink;
        let row_w       = usable_w * perspective;
        let x_offset    = params.margin_x + (usable_w - row_w) * 0.5;

        // Is this pixel within the row's x range?
        let local_x = fx - x_offset;
        if local_x < 0.0 || local_x > row_w { continue; }

        // Map pixel x to spectrum bin (floating-point for interpolation).
        let bin_f = local_x / row_w * f32(params.n_bins - 1u);
        let val   = clamp(sample_spectrum(row_idx, bin_f), 0.0, 1.0);
        let amp   = val * params.amp_scale;
        let waveform_y = y_base - amp;

        // ── Occlusion fill: if pixel is between waveform and baseline ─────
        if fy >= waveform_y && fy <= y_base {
            color = bg;
        }

        // ── Glow + stroke: if pixel is near the waveform line ─────────────
        // Only draw if pixel is at or above the baseline.
        if fy <= y_base {
            let dist = abs(fy - waveform_y);

            // Glow radius shrinks with depth.
            let glow_radius = 3.5 - depth_frac * 1.5;

            if dist < glow_radius {
                // Stroke color: front=bright cyan, back=dark teal.
                let front_color = vec3(0.08, 0.90, 1.00);
                let back_color  = vec3(0.02, 0.39, 0.55);
                let stroke_color = lerp3(front_color, back_color, depth_frac);

                // Intensity: bright core, soft falloff.
                let core_width = 1.2 - depth_frac * 0.3;
                var intensity: f32;
                if dist < core_width {
                    // Core: near-full brightness.
                    intensity = 1.0 - dist / core_width * 0.2;
                } else {
                    // Glow falloff.
                    intensity = (1.0 - (dist - core_width) / (glow_radius - core_width)) * 0.5;
                }
                intensity *= (1.0 - depth_frac * 0.4);

                color = lerp3(color, stroke_color, clamp(intensity, 0.0, 1.0));
            }
        }
    }

    rgba_out[py * params.width + px] = pack_rgba(vec4(color, 1.0));
}
"#;

pub struct RidgePlotGpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::ComputePipeline,
    params_buf: wgpu::Buffer,
    history_buf: wgpu::Buffer,
    rgba_buf: wgpu::Buffer,
    readback_buf: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    params: RidgeParams,
}

impl RidgePlotGpu {
    pub fn new(device: wgpu::Device, queue: wgpu::Queue) -> anyhow::Result<Self> {
        let params = RidgeParams {
            width: RIDGE_W,
            height: RIDGE_H,
            n_rows: 0,
            n_bins: RIDGE_BINS as u32,
            row_step: 7.0,
            amp_scale: 50.0,
            margin_x: 24.0,
            margin_bottom: 16.0,
            bg_r: 0.027, // #07
            bg_g: 0.031, // #08
            bg_b: 0.059, // #0f
            perspective_shrink: 0.15,
        };

        let params_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("ridge-params"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let history_size = (HISTORY_FLOATS * 4) as u64;
        let rgba_size = (RIDGE_PIXELS * 4) as u64;

        let history_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ridge-history"),
            size: history_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let rgba_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ridge-rgba"),
            size: rgba_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let readback_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ridge-readback"),
            size: rgba_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ridge-shader"),
            source: wgpu::ShaderSource::Wgsl(RIDGE_SHADER.into()),
        });

        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ridge-bgl"),
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
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
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

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ridge-bg"),
            layout: &bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: params_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: history_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: rgba_buf.as_entire_binding(),
                },
            ],
        });

        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ridge-pl"),
            bind_group_layouts: &[&bgl],
            immediate_size: 0,
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("ridge-pipeline"),
            layout: Some(&pl),
            module: &shader,
            entry_point: Some("ridge_render"),
            compilation_options: Default::default(),
            cache: None,
        });

        Ok(Self {
            device,
            queue,
            pipeline,
            params_buf,
            history_buf,
            rgba_buf,
            readback_buf,
            bind_group,
            params,
        })
    }

    /// Render the ridge plot from a flat spectrum history.
    ///
    /// `history_flat` must be `n_rows × RIDGE_BINS` f32 values where
    /// index 0 is the oldest (back) row and `n_rows-1` is the newest (front).
    ///
    /// Returns the RGBA pixel data as packed u32 (R in LSB).
    pub fn render(&mut self, history_flat: &[f32], n_rows: usize) -> Vec<u32> {
        let n_rows = n_rows.min(RIDGE_ROWS);
        self.params.n_rows = n_rows as u32;

        // Upload params.
        self.queue
            .write_buffer(&self.params_buf, 0, bytemuck::bytes_of(&self.params));

        // Upload history — zero-pad if smaller than max.
        if !history_flat.is_empty() {
            let upload_len = history_flat.len().min(HISTORY_FLOATS);
            self.queue.write_buffer(
                &self.history_buf,
                0,
                bytemuck::cast_slice(&history_flat[..upload_len]),
            );
        }

        // Dispatch compute — 8×8 workgroups covering RIDGE_W × RIDGE_H pixels.
        let wg_x = (RIDGE_W + 7) / 8;
        let wg_y = (RIDGE_H + 7) / 8;

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("ridge-enc"),
            });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("ridge-pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.dispatch_workgroups(wg_x, wg_y, 1);
        }

        let rgba_bytes = (RIDGE_PIXELS * 4) as u64;
        encoder.copy_buffer_to_buffer(&self.rgba_buf, 0, &self.readback_buf, 0, rgba_bytes);
        self.queue.submit(std::iter::once(encoder.finish()));

        // Readback.
        let slice = self.readback_buf.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| {
            tx.send(r).unwrap();
        });
        self.device.poll(wgpu::PollType::wait_indefinitely()).ok();
        rx.recv().unwrap().expect("Ridge plot readback failed");

        let data = bytemuck::cast_slice::<u8, u32>(&slice.get_mapped_range()).to_vec();
        self.readback_buf.unmap();
        data
    }

    pub fn width(&self) -> u32 {
        RIDGE_W
    }
    pub fn height(&self) -> u32 {
        RIDGE_H
    }
}
