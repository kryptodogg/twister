use std::sync::Arc;
use tokio::runtime::Runtime;
use twister::visualization::mesh_shaders::{WavefieldRenderer, CameraUniform, InstanceData};

fn main() {
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        let instance = wgpu::Instance::default();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: None,
            })
            .await
            .expect("Failed to find wgpu adapter");

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    // Note: SUBGROUP was requested, make sure it's available or fallback
                    required_features: wgpu::Features::SUBGROUP,
                    required_limits: wgpu::Limits::default(),
                    label: None,
                    memory_hints: wgpu::MemoryHints::Performance,
                },
                None,
            )
            .await
            .expect("Failed to create wgpu device");

        let device = Arc::new(device);
        let queue = Arc::new(queue);

        let width = 1024;
        let height = 1024;
        let mut renderer = WavefieldRenderer::new(device.clone(), queue.clone(), width, height);

        let camera = CameraUniform {
            view_proj: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            camera_pos: [0.0, 0.0, -10.0],
            _padding1: 0,
            viewport_width: width,
            viewport_height: height,
            _padding2: [0, 0],
        };

        let instances = vec![
            InstanceData {
                position: [0.0, 0.0, 0.0],
                intensity: 0.8,
            },
            InstanceData {
                position: [2.0, 1.0, 0.0],
                intensity: 0.5,
            },
        ];

        renderer.update_camera(&camera);
        renderer.update_instances(&instances);
        renderer.render();

        let data = renderer.read_texture().await.expect("Failed to read texture");
        println!("Successfully read texture. Length: {} bytes", data.len());
    });
}
