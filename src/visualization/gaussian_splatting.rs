//! Gaussian Splatting: 3D Point Cloud Visualization
//!
//! GPU-accelerated rendering of point clouds using 3D Gaussian kernels
//! with accumulation and heat map tonemap for intensity visualization.
//!
//! **Algorithm Overview**:
//! For each pixel in viewport, accumulates contributions from all points:
//!   I_pixel = Σ_p [ intensity_p * exp(-0.5 * dist_p² / σ²) ]
//!
//! Where dist_p is the distance from pixel to projected point position.
//! The result is then tonemapped through a perceptually-uniform heat map
//! (Blue → Cyan → Green → Yellow → Red → White).

use std::sync::Arc;
use wgpu::TextureFormat;

use crate::gpu_shared::GpuShared;

const WORKGROUP_SIZE: u32 = 16;

const GAUSSIAN_SPLAT_WGSL: &str = r#"
struct Point {
    az: f32,      // Azimuth: -π to π
    el: f32,      // Elevation: -π/2 to π/2
    freq: f32,    // Frequency: normalized 0-1
    intensity: f32, // Intensity: 0-1
    ts: f32,      // Timestamp: normalized 0-1
    conf: f32,    // Confidence: 0-1
}

struct Uniforms {
    width: u32,
    height: u32,
    point_count: u32,
    sigma: f32,
    time: f32,
}

@group(0) @binding(0) var<storage, read> points: array<Point>;
@group(0) @binding(1) var<uniform> uniforms: Uniforms;
@group(0) @binding(2) var<storage, read_write> output: array<vec4<f32>>;

fn project_to_screen(az: f32, el: f32, freq: f32) -> vec2<f32> {
    // Project 3D spherical coordinates to 2D screen
    // azimuth maps to x, elevation to y, frequency affects z-depth
    let x = az / 3.14159265;  // -1 to 1
    let y = el / 1.57079633;  // -1 to 1
    // Map to screen coordinates (0 to width/height)
    let screen_x = (x * 0.5 + 0.5) * f32(uniforms.width);
    let screen_y = (1.0 - (y * 0.5 + 0.5)) * f32(uniforms.height); // Flip Y
    return vec2<f32>(screen_x, screen_y);
}

fn heatmap_tonemap(intensity: f32) -> vec3<f32> {
    // Blue → Cyan → Green → Yellow → Red → White
    let t = clamp(intensity, 0.0, 1.0);
    var r: f32 = 0.0;
    var g: f32 = 0.0;
    var b: f32 = 0.0;
    
    if (t < 0.25) {
        // Blue to Cyan
        let s = t / 0.25;
        r = 0.0;
        g = s;
        b = 1.0;
    } else if (t < 0.5) {
        // Cyan to Green
        let s = (t - 0.25) / 0.25;
        r = 0.0;
        g = 1.0;
        b = 1.0 - s;
    } else if (t < 0.75) {
        // Green to Yellow
        let s = (t - 0.5) / 0.25;
        r = s;
        g = 1.0;
        b = 0.0;
    } else if (t < 1.0) {
        // Yellow to Red
        let s = (t - 0.75) / 0.25;
        r = 1.0;
        g = 1.0 - s;
        b = 0.0;
    } else {
        // Red to White (clamped)
        let excess = min(t - 1.0, 1.0);
        r = 1.0;
        g = excess;
        b = excess;
    }
    return vec3<f32>(r, g, b);
}

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x = global_id.x;
    let y = global_id.y;
    
    if (x >= uniforms.width || y >= uniforms.height) {
        return;
    }
    
    let pixel_idx = y * uniforms.width + x;
    var accumulated_intensity: f32 = 0.0;
    
    // Accumulate Gaussian contributions from all points
    for (var i = 0u; i < uniforms.point_count; i = i + 1u) {
        let p = points[i];
        
        // Project point to screen space
        let proj = project_to_screen(p.az, p.el, p.freq);
        
        // Calculate distance from pixel to projected point
        let dx = f32(x) - proj.x;
        let dy = f32(y) - proj.y;
        let dist_sq = dx * dx + dy * dy;
        
        // Gaussian kernel: intensity * exp(-0.5 * dist² / σ²)
        let gaussian = p.intensity * exp(-0.5 * dist_sq / (uniforms.sigma * uniforms.sigma));
        
        // Weight by confidence
        accumulated_intensity += gaussian * p.conf;
    }
    
    // Apply heat map tonemap
    let color = heatmap_tonemap(accumulated_intensity);
    
    // Store RGBA output
    output[pixel_idx] = vec4<f32>(color.r, color.g, color.b, 1.0);
}
"#;

pub struct GaussianSplatRenderer {
    shared: Arc<GpuShared>,
    point_buffer: wgpu::Buffer,
    uniform_buffer: wgpu::Buffer,
    output_texture: wgpu::Texture,
    output_buffer: wgpu::Buffer,
    pipeline: wgpu::ComputePipeline,
    bind_group: wgpu::BindGroup,
    viewport_width: u32,
    viewport_height: u32,
    gaussian_sigma: f32,
    max_point_count: usize,
    debug_mode: bool,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct SplatUniforms {
    width: u32,
    height: u32,
    point_count: u32,
    sigma: f32,
    time: f32,
}

impl GaussianSplatRenderer {
    pub fn new(
        shared: Arc<GpuShared>,
        viewport_width: u32,
        viewport_height: u32,
        max_point_count: usize,
    ) -> Self {
        let device = &shared.device;

        let point_buffer_size = max_point_count * std::mem::size_of::<[f32; 6]>();
        let point_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("gaussian_splat_points"),
            size: point_buffer_size as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("gaussian_splat_uniforms"),
            size: std::mem::size_of::<SplatUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let output_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("gaussian_splat_output"),
            size: wgpu::Extent3d {
                width: viewport_width,
                height: viewport_height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: TextureFormat::Rgba32Float,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        let output_buffer_size = viewport_width as u64
            * viewport_height as u64
            * 4u64
            * std::mem::size_of::<f32>() as u64;
        let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("gaussian_splat_readback"),
            size: output_buffer_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("gaussian_splat_shader"),
            source: wgpu::ShaderSource::Wgsl(GAUSSIAN_SPLAT_WGSL.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("gaussian_splat_pipeline"),
            bind_group_layouts: &[],
            immediate_size: 0,
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("gaussian_splat_compute"),
            layout: Some(&pipeline_layout),
            module: &shader_module,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("gaussian_splat_bind_group"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &point_buffer,
                        offset: 0,
                        size: None,
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &uniform_buffer,
                        offset: 0,
                        size: None,
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(
                        &output_texture.create_view(&wgpu::TextureViewDescriptor::default()),
                    ),
                },
            ],
        });

        Self {
            shared,
            point_buffer,
            uniform_buffer,
            output_texture,
            output_buffer,
            pipeline,
            bind_group,
            viewport_width,
            viewport_height,
            gaussian_sigma: 0.1,
            max_point_count,
            debug_mode: false,
        }
    }

    pub fn render(&mut self, points: &[(f32, f32, f32, f32, f32, f32)]) -> Vec<u8> {
        if points.is_empty() {
            if self.debug_mode {
                eprintln!("[GaussianSplat] No points to render");
            }
            return vec![0u8; (self.viewport_width * self.viewport_height * 4) as usize];
        }

        let point_count = points.len().min(self.max_point_count);
        let device = &self.shared.device;
        let queue = &self.shared.queue;

        if self.debug_mode {
            eprintln!(
                "[GaussianSplat] Rendering {} points to {}x{} viewport",
                point_count, self.viewport_width, self.viewport_height
            );
        }

        // Pack point data: [az, el, freq, intensity, ts, conf]
        let mut point_data = vec![0.0f32; point_count * 6];
        for (i, p) in points.iter().take(point_count).enumerate() {
            point_data[i * 6 + 0] = p.0; // azimuth
            point_data[i * 6 + 1] = p.1; // elevation
            point_data[i * 6 + 2] = p.2; // frequency
            point_data[i * 6 + 3] = p.3; // intensity
            point_data[i * 6 + 4] = p.4; // timestamp
            point_data[i * 6 + 5] = p.5; // confidence
        }

        queue.write_buffer(&self.point_buffer, 0, bytemuck::cast_slice(&point_data));

        let uniforms = SplatUniforms {
            width: self.viewport_width,
            height: self.viewport_height,
            point_count: point_count as u32,
            sigma: self.gaussian_sigma,
            time: 0.0,
        };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniforms));

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("gaussian_splat_encoder"),
        });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("gaussian_splat_compute_pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.pipeline);
            compute_pass.set_bind_group(0, &self.bind_group, &[]);

            let workgroup_x = (self.viewport_width + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE;
            let workgroup_y = (self.viewport_height + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE;
            compute_pass.dispatch_workgroups(workgroup_x, workgroup_y, 1);
        }

        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &self.output_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &self.output_buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(
                        self.viewport_width * 4 * std::mem::size_of::<f32>() as u32,
                    ),
                    rows_per_image: Some(self.viewport_height),
                },
            },
            wgpu::Extent3d {
                width: self.viewport_width,
                height: self.viewport_height,
                depth_or_array_layers: 1,
            },
        );

        queue.submit([encoder.finish()]);

        let buffer_slice = self.output_buffer.slice(..);
        buffer_slice.map_async(wgpu::MapMode::Read, |_| {});
        let _ = device.poll(wgpu::PollType::wait_indefinitely());

        let mapped = buffer_slice.get_mapped_range();
        let float_data: &[f32] = bytemuck::cast_slice(&mapped);

        let mut image_data = vec![0u8; (self.viewport_width * self.viewport_height * 4) as usize];
        for (i, &f) in float_data.iter().enumerate() {
            let c = (f.clamp(0.0, 1.0) * 255.0) as u8;
            image_data[i] = c;
        }
        drop(mapped);
        self.output_buffer.unmap();

        image_data
    }

    pub fn resize(&mut self, new_width: u32, new_height: u32) {
        if new_width == 0 || new_height == 0 {
            eprintln!("Warning: Cannot resize to {}x{}", new_width, new_height);
            return;
        }

        let device = &self.shared.device;

        self.viewport_width = new_width;
        self.viewport_height = new_height;

        // Recreate output texture and buffer with new dimensions
        let output_buffer_size =
            new_width as u64 * new_height as u64 * 4 * std::mem::size_of::<f32>() as u64;

        self.output_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("gaussian_splat_output"),
            size: wgpu::Extent3d {
                width: new_width,
                height: new_height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: TextureFormat::Rgba32Float,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        self.output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("gaussian_splat_readback"),
            size: output_buffer_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Recreate bind group with new texture
        self.bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("gaussian_splat_bind_group"),
            layout: &self.pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &self.point_buffer,
                        offset: 0,
                        size: None,
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &self.uniform_buffer,
                        offset: 0,
                        size: None,
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(
                        &self
                            .output_texture
                            .create_view(&wgpu::TextureViewDescriptor::default()),
                    ),
                },
            ],
        });

        if self.debug_mode {
            eprintln!("[GaussianSplat] Resized to {}x{}", new_width, new_height);
        }
    }

    pub fn set_gaussian_sigma(&mut self, sigma: f32) {
        if sigma <= 0.0 {
            eprintln!("Warning: Sigma must be > 0, got {}", sigma);
            return;
        }
        self.gaussian_sigma = sigma;
    }

    pub fn set_debug_mode(&mut self, enabled: bool) {
        self.debug_mode = enabled;
    }

    pub fn viewport_size(&self) -> (u32, u32) {
        (self.viewport_width, self.viewport_height)
    }

    pub fn gaussian_sigma(&self) -> f32 {
        self.gaussian_sigma
    }
}

pub fn intensity_to_rgb(intensity: f32) -> (u8, u8, u8) {
    let clamped = intensity.max(0.0);
    let (r, g, b) = if clamped < 0.25 {
        let t = clamped / 0.25;
        (0.0, t, 1.0)
    } else if clamped < 0.5 {
        let t = (clamped - 0.25) / 0.25;
        (0.0, 1.0, 1.0 - t)
    } else if clamped < 0.75 {
        let t = (clamped - 0.5) / 0.25;
        (t, 1.0, 0.0)
    } else if clamped < 1.0 {
        let t = (clamped - 0.75) / 0.25;
        (1.0, 1.0 - t, 0.0)
    } else {
        let excess = (clamped - 1.0).min(1.0);
        (1.0, excess, excess)
    };
    ((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_colormap_blue_region() {
        let (r, _g, b) = intensity_to_rgb(0.0);
        assert_eq!((r, b), (0, 255));
    }

    #[test]
    fn test_colormap_red_region() {
        let (r, _, _) = intensity_to_rgb(1.0);
        assert_eq!(r, 255);
    }

    #[test]
    fn test_colormap_white_region() {
        let (r, g, b) = intensity_to_rgb(2.0);
        assert_eq!((r, g, b), (255, 255, 255));
    }
}
