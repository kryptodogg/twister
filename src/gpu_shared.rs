// src/gpu_shared.rs — Singleton wgpu Device / Queue  (wgpu v28, Windows 11 DX12)
//
// One GpuShared is created at startup and Arc-cloned into every GPU engine.
// wgpu v28: request_adapter() returns Result<Adapter, RequestAdapterError>,
// NOT Option<Adapter>.  Use .map_err() to convert the error.

use anyhow::Context;
use std::sync::Arc;

pub struct GpuShared {
    pub device:       wgpu::Device,
    pub queue:        wgpu::Queue,
    pub adapter_info: wgpu::AdapterInfo,
}

impl GpuShared {
    pub fn new() -> anyhow::Result<Arc<Self>> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::DX12,
            ..Default::default()
        });

        // wgpu v28: request_adapter returns Result — use .map_err, not .ok_or_else
        let adapter = pollster::block_on(instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference:       wgpu::PowerPreference::HighPerformance,
                compatible_surface:     None,
                force_fallback_adapter: false,
            },
        ))
        .map_err(|e| anyhow::anyhow!("No DX12 adapter found: {e:?}"))?;

        let info = adapter.get_info();
        println!(
            "[GPU] {} ({:?}) via {:?}",
            info.name, info.device_type, info.backend
        );

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label:             Some("twister-shared-device"),
                required_features: wgpu::Features::empty(),
                required_limits:   wgpu::Limits::default(),
                ..Default::default()
            },
        ))
        .context("GPU device creation failed")?;

        Ok(Arc::new(Self { device, queue, adapter_info: info }))
    }
}
