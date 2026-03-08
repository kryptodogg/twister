//! Gaussian Splatting Benchmark (256-byte alignment, GPU timestamps)
//!
//! Measures pure GPU compute time for rendering 10,000 particles with
//! Struct-of-Arrays memory layout optimized for RDNA2 Wave64 coalescing.
//!
//! Run with:
//!   cargo run --example gaussian_splat_bench --release

use twister::visualization::gaussian_splatting_optimized::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("[GaussianSplat Bench] Starting 256-byte aligned SoA benchmark...");

    // Initialize wgpu with timestamp query support
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::PRIMARY,
        ..Default::default()
    });

    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: None,
        force_fallback_adapter: false,
    }))
    .map_err(|e| format!("No adapter found: {e:?}"))?;

    println!("[GaussianSplat Bench] Adapter: {} ({:?})",
             adapter.get_info().name,
             adapter.get_info().backend);

    // Request device with TIMESTAMP_QUERY feature
    let (device, queue) = pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("gaussian_splat_bench_device"),
            required_features: wgpu::Features::TIMESTAMP_QUERY,
            required_limits: wgpu::Limits::default(),
            ..Default::default()
        }
    ))?;

    println!("[GaussianSplat Bench] Device initialized with TIMESTAMP_QUERY support");

    // Create renderer with 256-byte alignment
    let config = GaussianSplattingConfig {
        max_particles: 10_000,
        width: 1024,
        height: 1024,
        sigma: 1.0,
    };

    let renderer = GaussianSplattingRenderer::new(&device, &queue, Some(config))?;
    println!("[GaussianSplat Bench] Renderer created: 256-byte aligned SoA buffers");

    // Generate test particle data (10,000 particles)
    let mut particles = Vec::new();
    for i in 0..10_000 {
        let angle = (i as f32 / 10_000.0) * std::f32::consts::TAU;
        let radius = (i as f32 / 10_000.0) * std::f32::consts::PI;

        particles.push(Particle {
            azimuth_rad: angle,
            elevation_rad: radius,
            frequency_hz: 1_000_000.0 + (i as f32 * 100.0),
            intensity: (i as f32 / 10_000.0).sin().abs(),
        });
    }

    println!("[GaussianSplat Bench] Generated {} particles", particles.len());

    // Upload to GPU
    renderer.upload_particles(&particles)?;
    println!("[GaussianSplat Bench] Particles uploaded to GPU");

    // Run benchmark (warm-up pass)
    println!("[GaussianSplat Bench] Warm-up pass...");
    renderer.render(10_000)?;

    // Measure pass (GPU timestamps captured automatically)
    println!("[GaussianSplat Bench] Measurement pass (GPU timestamps enabled)...");
    renderer.render(10_000)?;

    println!("[GaussianSplat Bench] Benchmark complete!");
    println!("[GaussianSplat Bench] GPU execution time printed above ↑");

    Ok(())
}
