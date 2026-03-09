use std::error::Error;
use twister::visualization::data_contracts::VoxelGrid;
use twister::visualization::ray_tracing_renderer::{RayTracingRenderer, Camera};
use twister::visualization::lumen_global_illumination::LumenGI;
use twister::visualization::volumetric_lighting::VolumetricLighting;
use twister::visualization::tone_mapping::{tone_map_reinhard, tone_map_aces};

async fn setup_device() -> Result<(wgpu::Device, wgpu::Queue), Box<dyn Error>> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::VULKAN | wgpu::Backends::DX12,
        ..Default::default()
    });

    let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        force_fallback_adapter: false,
        compatible_surface: None,
    }).await.ok_or("Failed to find suitable adapter")?;

    let required_features = wgpu::Features::RAY_QUERY | wgpu::Features::RAY_TRACING_ACCELERATION_STRUCTURE;

    let (device, queue) = adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("Megalights Device"),
            required_features,
            required_limits: wgpu::Limits::default(),
        },
        None,
    ).await.map_err(|e| format!("Failed to create device: {}", e))?;

    Ok((device, queue))
}

#[tokio::test]
async fn test_pipeline_creation() {
    let setup = setup_device().await;
    if let Ok((device, queue)) = setup {
        let renderer = RayTracingRenderer::new(&device, &queue);
        let _cmd_buf = renderer.render_megalights();
    }
}

#[tokio::test]
async fn test_lumen_gi_creation() {
    let setup = setup_device().await;
    if let Ok((device, _)) = setup {
        let gi = LumenGI::new(&device, 100.0);
        assert_eq!(gi.probe_grid.len(), 512);
    }
}

#[tokio::test]
async fn test_lumen_gi_sampling() {
    let setup = setup_device().await;
    if let Ok((device, queue)) = setup {
        let mut gi = LumenGI::new(&device, 100.0);
        let grid = VoxelGrid::new(10, 10, 10);

        gi.update_probes(&queue, &grid, &device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        }));

        let irradiance = gi.sample_indirect([50.0, 50.0, 50.0]);
        assert_eq!(irradiance, [0.0, 0.0, 0.0]);
    }
}

#[tokio::test]
async fn test_volumetric_lighting_creation() {
    let setup = setup_device().await;
    if let Ok((device, _)) = setup {
        let _vol = VolumetricLighting::new(&device);
    }
}

#[test]
fn test_tone_mapping() {
    let linear = [0.5, 0.5, 0.5];
    let reinhard = tone_map_reinhard(linear, 1.0, 1.0);
    assert!(reinhard[0] > 0.0 && reinhard[0] < 1.0);

    let aces = tone_map_aces(linear);
    assert!(aces[0] > 0.0 && aces[0] < 1.0);
}
