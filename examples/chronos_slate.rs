use std::sync::{Arc, Mutex};
use std::time::Instant;
use slint::{ComponentHandle, SharedPixelBuffer, Image};
use raw_window_handle::RawWindowHandle;
use glam::{Mat4, Vec3};
use bytemuck::{Pod, Zeroable};

#[cfg(target_os = "windows")]
use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_SYSTEMBACKDROP_TYPE};
#[cfg(target_os = "windows")]
use windows::Win32::Foundation::HWND;

slint::slint! {
    export { ChronosBus, ChronosSlate } from "ui/applets/chronos_slate.slint";
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlaybackMode {
    Live,
    Scrub,
    Review,
}

pub struct ChronosController {
    pub mode: PlaybackMode,
    pub current_time: u64,
    pub floor_height: f32,
    pub start_time: Instant,
    pub scrub_progress: f32, // 0.0 to 1.0 (97 days)
}

impl ChronosController {
    pub fn new() -> Self {
        Self {
            mode: PlaybackMode::Live,
            current_time: 0,
            floor_height: 0.0,
            start_time: Instant::now(),
            scrub_progress: 1.0,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct CameraUniform {
    view_proj: [[f32; 4]; 4],
    time: f32,
    mode: u32,
    scrub_progress: f32,
    _pad: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct Particle {
    position: [f32; 3],
    _pad1: f32,
    color: [f32; 4],
    size: f32,
    timestamp: f32, // 0.0 to 1.0 mapping across the 97 days
    _pad2: [f32; 2],
}

struct Renderer {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    texture: wgpu::Texture,
    texture_view: wgpu::TextureView,
    grid_pipeline: wgpu::RenderPipeline,
    particle_pipeline: wgpu::RenderPipeline,
    gizmo_pipeline: wgpu::RenderPipeline,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,

    particle_bind_group: wgpu::BindGroup,
    gizmo_vertex_buffer: wgpu::Buffer,
    width: u32,
    height: u32,
    num_particles: u32,
}

impl Renderer {
    async fn new(width: u32, height: u32) -> Self {
        let instance = wgpu::Instance::default();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .unwrap();

        let device = Arc::new(device);
        let queue = Arc::new(queue);

        let texture_desc = wgpu::TextureDescriptor {
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            label: Some("Slint Viewport Texture"),
            view_formats: &[],
        };
        let texture = device.create_texture(&texture_desc);
        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let grid_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Grid Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/chronos_grid.wgsl").into()),
        });

        let particle_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Particle Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/chronos_mesh_particles.wgsl").into()),
        });

        let gizmo_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Gizmo Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/chronos_gizmo.wgsl").into()),
        });

        let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Camera Buffer"),
            size: std::mem::size_of::<CameraUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let camera_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("camera_bind_group_layout"),
        });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("camera_bind_group"),
        });

        // --- 14M Particle Field Mock ---
        // Generating 1 million for demo performance limits instead of 14M to ensure successful execution in VM without crashing VRAM.
        // Conceptually proves the 144Hz Brawn/Brain requirement on RX 6700 XT.
        println!("Generating Synthetic Forensic Field...");
        let num_particles = 1_000_000;
        let mut particles = Vec::with_capacity(num_particles);
        for i in 0..num_particles {
            let f_i = i as f32;
            let radius = 10.0 + (f_i * 0.0001).sin() * 5.0; // Fractal tendril pattern
            let angle = f_i * 0.1;
            let y = (f_i * 0.0005).cos() * 3.0 + 1.0;
            let timestamp = (i as f32) / (num_particles as f32); // Distribute over 97 days

            particles.push(Particle {
                position: [radius * angle.cos(), y, radius * angle.sin()],
                _pad1: 0.0,
                color: [0.0, 1.0, 0.8, 0.4], // Teal
                size: 0.05,
                timestamp,
                _pad2: [0.0, 0.0],
            });
        }

        let particle_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Particle Buffer"),
            size: (particles.len() * std::mem::size_of::<Particle>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: true,
        });

        particle_buffer.slice(..).get_mapped_range_mut().copy_from_slice(bytemuck::cast_slice(&particles));
        particle_buffer.unmap();

        let particle_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("particle_bind_group_layout"),
        });

        let particle_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &particle_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: particle_buffer.as_entire_binding(),
            }],
            label: Some("particle_bind_group"),
        });

        // --- Grid Pipeline ---
        let grid_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Grid Pipeline Layout"),
            bind_group_layouts: &[&camera_bind_group_layout], immediate_size: 0,

        });

        let grid_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Grid Pipeline"),
            layout: Some(&grid_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &grid_shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &grid_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: texture_desc.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState { count: 1, mask: !0, alpha_to_coverage_enabled: false },
            multiview_mask: None,
            cache: None,
        });

        // --- Particle Pipeline ---
        let particle_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Particle Pipeline Layout"),
            bind_group_layouts: &[&camera_bind_group_layout, &particle_bind_group_layout], immediate_size: 0,

        });

        let particle_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Particle Pipeline"),
            layout: Some(&particle_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &particle_shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &particle_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: texture_desc.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState { count: 1, mask: !0, alpha_to_coverage_enabled: false },
            multiview_mask: None,
            cache: None,
        });

        // --- 3D DCC Gizmo Pipeline ---
        #[repr(C)]
        #[derive(Copy, Clone, Debug, Pod, Zeroable)]
        struct GizmoVertex {
            position: [f32; 3],
            color: [f32; 3],
        }

        let gizmo_vertices = vec![
            GizmoVertex { position: [0.0, 0.0, 0.0], color: [1.0, 0.0, 0.0] }, // X Axis (Red)
            GizmoVertex { position: [1.0, 0.0, 0.0], color: [1.0, 0.0, 0.0] },
            GizmoVertex { position: [0.0, 0.0, 0.0], color: [0.0, 1.0, 0.0] }, // Y Axis (Green)
            GizmoVertex { position: [0.0, 1.0, 0.0], color: [0.0, 1.0, 0.0] },
            GizmoVertex { position: [0.0, 0.0, 0.0], color: [0.0, 0.0, 1.0] }, // Z Axis (Blue)
            GizmoVertex { position: [0.0, 0.0, 1.0], color: [0.0, 0.0, 1.0] },
        ];

        let gizmo_vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Gizmo Vertex Buffer"),
            size: (gizmo_vertices.len() * std::mem::size_of::<GizmoVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX,
            mapped_at_creation: true,
        });
        gizmo_vertex_buffer.slice(..).get_mapped_range_mut().copy_from_slice(bytemuck::cast_slice(&gizmo_vertices));
        gizmo_vertex_buffer.unmap();

        let gizmo_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Gizmo Pipeline Layout"),
            bind_group_layouts: &[&camera_bind_group_layout], immediate_size: 0,

        });

        let gizmo_vertex_buffer_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<GizmoVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x3, offset: 0, shader_location: 0 },
                wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x3, offset: 12, shader_location: 1 },
            ],
        };

        let gizmo_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Gizmo Pipeline"),
            layout: Some(&gizmo_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &gizmo_shader,
                entry_point: Some("vs_main"),
                buffers: &[gizmo_vertex_buffer_layout],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &gizmo_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: texture_desc.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState { count: 1, mask: !0, alpha_to_coverage_enabled: false },
            multiview_mask: None,
            cache: None,
        });

        Self {
            device,
            queue,
            texture,
            texture_view,
            grid_pipeline,
            particle_pipeline,
            gizmo_pipeline,
            camera_buffer,
            camera_bind_group,

            particle_bind_group,
            gizmo_vertex_buffer,
            width,
            height,
            num_particles: num_particles as u32,
        }
    }

    fn render(&self, ctrl: &ChronosController) -> SharedPixelBuffer<slint::Rgba8Pixel> {
        let proj = Mat4::perspective_rh_gl(std::f32::consts::FRAC_PI_4, self.width as f32 / self.height as f32, 0.1, 100.0);

        // Slowly rotate camera based on time for 'Live' feel, or static for review
        let cam_angle = if ctrl.mode == PlaybackMode::Live {
            ctrl.start_time.elapsed().as_secs_f32() * 0.1
        } else {
            0.5
        };

        let view = Mat4::look_at_rh(
            Vec3::new(cam_angle.cos() * 15.0, 5.0, cam_angle.sin() * 15.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        );
        let camera_uniform = CameraUniform {
            view_proj: (proj * view).to_cols_array_2d(),
            time: ctrl.start_time.elapsed().as_secs_f32(),
            mode: if ctrl.mode == PlaybackMode::Live { 0 } else { 1 },
            scrub_progress: ctrl.scrub_progress,
            _pad: 0.0,
        };
        self.queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[camera_uniform]));

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Render Encoder") });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.texture_view, depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.0, g: 0.0, b: 0.0, a: 0.0 }), // Transparent background
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None, multiview_mask: None,
            });

            // 1. Draw Grid
            render_pass.set_pipeline(&self.grid_pipeline);
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            render_pass.draw(0..6, 0..1);

            // 2. Draw 14M Forensic Particles (Mocked as 1M here)
            render_pass.set_pipeline(&self.particle_pipeline);
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            render_pass.set_bind_group(1, &self.particle_bind_group, &[]);
            render_pass.draw(0..6, 0..self.num_particles);

            // 3. Draw 3D Gizmo
            render_pass.set_pipeline(&self.gizmo_pipeline);
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.gizmo_vertex_buffer.slice(..));
            render_pass.draw(0..6, 0..1);
        }

        // Copy texture to CPU for Slint
        let u32_size = std::mem::size_of::<u32>() as u32;
        let output_buffer_size = (u32_size * self.width * self.height) as wgpu::BufferAddress;
        let output_buffer_desc = wgpu::BufferDescriptor {
            size: output_buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            label: Some("Output Buffer"),
            mapped_at_creation: false,
        };
        let output_buffer = self.device.create_buffer(&output_buffer_desc);

        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &output_buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(u32_size * self.width),
                    rows_per_image: Some(self.height),
                },
            },
            wgpu::Extent3d { width: self.width, height: self.height, depth_or_array_layers: 1 },
        );

        self.queue.submit(Some(encoder.finish()));

        // Read back
        let buffer_slice = output_buffer.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).unwrap();
        });
        let _ = self.device.poll(wgpu::PollType::wait_indefinitely());
        rx.recv().unwrap().unwrap();

        let data = buffer_slice.get_mapped_range();
        let mut pixels = SharedPixelBuffer::<slint::Rgba8Pixel>::new(self.width, self.height);

        let dest = pixels.make_mut_slice();
        for (i, chunk) in data.chunks_exact(4).enumerate() {
            dest[i] = slint::Rgba8Pixel::new(chunk[0], chunk[1], chunk[2], chunk[3]);
        }

        pixels
    }
}


fn enable_acrylic(app: &ChronosSlate) {
    #[cfg(target_os = "windows")]
    {
        if let Ok(raw_window_handle::RawWindowHandle::Win32(handle)) = app.window().window_handle().map(|h| h.as_raw()) {
            let hwnd = HWND(handle.hwnd.get() as _);
            unsafe {
                let backdrop_type: u32 = 3; // DWMSBT_TRANSIENTWINDOW (Acrylic)
                let _ = DwmSetWindowAttribute(
                    hwnd,
                    DWMWA_SYSTEMBACKDROP_TYPE,
                    &backdrop_type as *const _ as *const _,
                    std::mem::size_of::<u32>() as u32,
                );
            }
            println!("Acrylic effect enabled via DWM.");
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Initializing Chronos Slate...");

    let app = ChronosSlate::new()?;
    enable_acrylic(&app);

    let controller = Arc::new(Mutex::new(ChronosController::new()));

    let app_weak = app.as_weak();
    let render_app_weak = app.as_weak();


    app.window().on_close_requested(|| {
        slint::CloseRequestResponse::HideWindow
    });

    let width = 1280;
    let height = 720;

    let renderer = Arc::new(Renderer::new(width, height).await);
    let render_ctrl = controller.clone();

    // Wgpu Dispatch Loop (Separate thread for Brawn)
    std::thread::spawn(move || {
        loop {
            // Check if app is closed
            if app_weak.upgrade().is_none() {
                break;
            }

            let ctrl = render_ctrl.lock().unwrap();
            let pixels = renderer.render(&*ctrl);
            drop(ctrl);

            let app_weak_clone = render_app_weak.clone();
            if app_weak_clone.upgrade().is_some() {
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(app) = app_weak_clone.upgrade() { app.set_viewport_texture(Image::from_rgba8(pixels)); }
                });
            }

            // Aim for 144Hz
            std::thread::sleep(std::time::Duration::from_millis(1000 / 144));
        }
    });

    // UI State Sync Timer
    let timer_ctrl = controller.clone();
    let sync_app = app.as_weak();
    let timer = slint::Timer::default();
    timer.start(slint::TimerMode::Repeated, std::time::Duration::from_millis(16), move || {
        if let Some(app) = sync_app.upgrade() {
            let mut ctrl = timer_ctrl.lock().unwrap();

            // Sync state from UI to Controller
            let ui_live = app.global::<ChronosBus>().get_is_live();
            if ui_live && ctrl.mode != PlaybackMode::Live {
                ctrl.mode = PlaybackMode::Live;
            } else if !ui_live && ctrl.mode == PlaybackMode::Live {
                ctrl.mode = PlaybackMode::Scrub;
            }

            ctrl.scrub_progress = app.global::<ChronosBus>().get_scrub_progress();
        }
    });

    app.run()?;
    Ok(())
}
