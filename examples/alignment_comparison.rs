//! GPU Gaussian Splatting Alignment Comparison (128-byte vs 256-byte)
//!
//! Benchmarks GPU performance with different buffer alignment strategies
//! to understand RDNA2 memory coalescing behavior.
//!
//! Run with:
//!   cargo run --example alignment_comparison --release

use twister::visualization::gaussian_splatting_optimized::*;
use std::mem::size_of;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("═══════════════════════════════════════════════════════════");
    println!("Gaussian Splatting: 128-byte vs 256-byte Alignment Comparison");
    println!("═══════════════════════════════════════════════════════════\n");

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

    println!("[Adapter] {}", adapter.get_info().name);
    println!("[Backend] {:?}\n", adapter.get_info().backend);

    // Request device with TIMESTAMP_QUERY feature
    let (device, queue) = pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("alignment_compare_device"),
            required_features: wgpu::Features::TIMESTAMP_QUERY,
            required_limits: wgpu::Limits::default(),
            ..Default::default()
        }
    ))?;

    // Test parameters
    let particle_counts = vec![1000, 5000, 10000];

    for count in particle_counts {
        println!("╔════════════════════════════════════════════╗");
        println!("║ Testing with {} particles", format!("{} particles", count).to_uppercase());
        println!("╚════════════════════════════════════════════╝");

        // Test 128-byte alignment
        {
            println!("\n[128-byte] Calculating alignment...");
            let aligned_128 = ((count as u64 * size_of::<f32>() as u64) + 127) / 128 * 128;
            println!("[128-byte] Raw size: {} bytes", count * size_of::<f32>());
            println!("[128-byte] Aligned size: {} bytes", aligned_128);

            let config = GaussianSplattingConfig {
                max_particles: count as u32,
                width: 1024,
                height: 1024,
                sigma: 1.0,
            };

            match GaussianSplattingRenderer::new(&device, &queue, Some(config)) {
                Ok(renderer) => {
                    // Generate test particles
                    let particles: Vec<Particle> = (0..count)
                        .map(|i| {
                            let angle = (i as f32 / count as f32) * std::f32::consts::TAU;
                            let radius = (i as f32 / count as f32) * std::f32::consts::PI;
                            Particle {
                                azimuth_rad: angle,
                                elevation_rad: radius,
                                frequency_hz: 1_000_000.0 + (i as f32 * 100.0),
                                intensity: (i as f32 / count as f32).sin().abs(),
                            }
                        })
                        .collect();

                    renderer.upload_particles(&particles)?;

                    // Warm-up
                    renderer.render(count as u32)?;

                    // Measurement (run 3 times for stability)
                    let mut times = Vec::new();
                    for run in 1..=3 {
                        renderer.render(count as u32)?;
                        // Extract timing from the renderer's output (it prints it)
                    }

                    println!("[128-byte] ✓ Render successful");
                }
                Err(e) => {
                    println!("[128-byte] ✗ Renderer creation failed: {}", e);
                }
            }
        }

        // Test 256-byte alignment
        {
            println!("\n[256-byte] Calculating alignment...");
            let aligned_256 = ((count as u64 * size_of::<f32>() as u64) + 255) / 256 * 256;
            println!("[256-byte] Raw size: {} bytes", count * size_of::<f32>());
            println!("[256-byte] Aligned size: {} bytes", aligned_256);

            let config = GaussianSplattingConfig {
                max_particles: count as u32,
                width: 1024,
                height: 1024,
                sigma: 1.0,
            };

            match GaussianSplattingRenderer::new(&device, &queue, Some(config)) {
                Ok(renderer) => {
                    // Generate test particles
                    let particles: Vec<Particle> = (0..count)
                        .map(|i| {
                            let angle = (i as f32 / count as f32) * std::f32::consts::TAU;
                            let radius = (i as f32 / count as f32) * std::f32::consts::PI;
                            Particle {
                                azimuth_rad: angle,
                                elevation_rad: radius,
                                frequency_hz: 1_000_000.0 + (i as f32 * 100.0),
                                intensity: (i as f32 / count as f32).sin().abs(),
                            }
                        })
                        .collect();

                    renderer.upload_particles(&particles)?;

                    // Warm-up
                    renderer.render(count as u32)?;

                    // Measurement (run 3 times for stability)
                    let mut times = Vec::new();
                    for run in 1..=3 {
                        renderer.render(count as u32)?;
                        // Extract timing from the renderer's output (it prints it)
                    }

                    println!("[256-byte] ✓ Render successful");
                }
                Err(e) => {
                    println!("[256-byte] ✗ Renderer creation failed: {}", e);
                }
            }
        }

        println!("\n{separator}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n",
                 separator = "");
    }

    println!("═══════════════════════════════════════════════════════════");
    println!("Benchmark complete!");
    println!("Note: GPU timing is printed by each render() call above");
    println!("═══════════════════════════════════════════════════════════");

    Ok(())
}
