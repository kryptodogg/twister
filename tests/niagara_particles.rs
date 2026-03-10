use std::sync::Arc;
use tokio::runtime::Runtime;
use twister::gpu_shared::GpuShared;
use twister::visualization::particle_renderer::ParticleRenderer;

#[test]
fn test_mesh_shader_compilation() {
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        // Initialize real GPU request requesting MESH_SHADER
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN | wgpu::Backends::DX12,
            ..Default::default()
        });

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .expect("Failed to find wgpu adapter");

        // Request Mesh Shaders (crucial for VI.2)
        let mut required_features = wgpu::Features::empty();
        required_features.insert(wgpu::Features::MESH_SHADER);

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("niagara_test_device"),
                    required_features,
                    required_limits: wgpu::Limits::default(),
                },
                None,
            )
            .await
            .expect("Failed to create device with mesh shaders enabled. Is the driver up to date?");

        let shared = Arc::new(GpuShared {
            device,
            queue,
        });

        // Initialize pipeline (proves WGSL compiles and pipeline config is valid)
        let _renderer = ParticleRenderer::new(shared, 1_000_000);
    });
}
