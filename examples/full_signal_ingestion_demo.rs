// examples/full_signal_ingestion_demo.rs — End-to-End Signal Ingestion Demo
//
// Demonstrates the complete Track B pipeline:
//   RTL-SDR → IQ Dispatch → DMA → GPU FFT → V-Buffer → Context Window
//
// Run with: cargo run --example full_signal_ingestion_demo
//
// Requirements:
// - RTL-SDR device plugged in (or mock mode)
// - Vulkan-capable GPU (AMD RX 6700 XT or equivalent)

use std::sync::Arc;
use tokio::time::{timeout, Duration};

use twister::app_state::DirtyFlags;
use twister::dispatch::iq_dispatch::IqDispatchLoop;
use twister::hardware_io::device_manager::DeviceManager;
use twister::hardware_io::dma_vbuffer::IqDmaGateway;
use twister::visualization::stft_pipeline::StftProcessor;
use twister::vbuffer::GpuVBuffer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("📡 Track B: Signal Ingestion Demo");
    println!("================================");

    // Step 1: Initialize wgpu device
    println!("\n[1/5] Initializing wgpu device...");
    let (device, queue) = create_wgpu_device().await;
    let device_arc = Arc::new(device);
    let queue_arc = Arc::new(queue);

    // Step 2: Create device manager
    println!("[2/5] Creating device manager...");
    let dirty_flags = Arc::new(DirtyFlags::new());
    let device_manager = Arc::new(DeviceManager::new(dirty_flags));

    // Step 3: Create DMA gateway
    println!("[3/5] Creating DMA gateway...");
    let dma_gateway = Arc::new(std::sync::Mutex::new(
        IqDmaGateway::new(Arc::clone(&device_arc), Arc::clone(&queue_arc), 64)
    ));

    // Step 4: Create STFT processor
    println!("[4/5] Creating STFT processor...");
    let mut stft_processor = StftProcessor::new(
        Arc::clone(&device_arc),
        Arc::clone(&queue_arc),
    )?;

    println!("✅ Pipeline initialized");
    println!("   - FFT size: 512 complex samples");
    println!("   - Frequency bins: 512");
    println!("   - V-Buffer depth: 512 frames (10.7s context)");

    // Step 5: Run demo (mock mode if no device)
    println!("\n[5/5] Running signal ingestion demo (10 seconds)...");
    
    let mut dispatch = IqDispatchLoop::new(
        Arc::clone(&device_manager),
        Arc::clone(&dma_gateway),
    );

    // Run for 10 seconds with timeout
    let demo_result = timeout(Duration::from_secs(10), dispatch.run()).await;

    match demo_result {
        Ok(result) => {
            match result {
                Ok(_) => println!("✅ Demo completed successfully"),
                Err(e) => println!("⚠️ Demo ended with error: {}", e),
            }
        }
        Err(_) => {
            println!("⏱️ Demo timeout (10s reached)");
        }
    }

    dispatch.stop();

    // Report statistics
    println!("\n📊 Statistics:");
    println!("   Frames processed: {}", dispatch.frame_count());
    println!("   Frames dropped: {}", dispatch.dropped_frames());
    println!("   DMA offset: {} bytes", dma_gateway.lock().unwrap().write_offset());

    // Test V-Buffer context window
    println!("\n🧪 Testing V-Buffer context window...");
    let vbuffer = stft_processor.vbuffer();
    println!("   Available frames: {}", vbuffer.available_frames());
    println!("   Buffer ready: {}", vbuffer.ready(10));

    println!("\n✅ Track B Signal Ingestion Demo Complete!");
    println!("========================================");

    Ok(())
}

/// Helper to create a wgpu device
async fn create_wgpu_device() -> (wgpu::Device, wgpu::Queue) {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::VULKAN,
        ..Default::default()
    });

    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .await
        .expect("Failed to find an appropriate adapter");

    println!("   Adapter: {:?}", adapter.get_info().name);

    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: Some("Signal Ingestion Demo Device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::Performance,
            },
            None,
        )
        .await
        .expect("Failed to create device");

    (device, queue)
}
