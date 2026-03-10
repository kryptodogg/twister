sed -i 's/\.request_device(&wgpu::DeviceDescriptor::default(), None)/.request_device(\&wgpu::DeviceDescriptor::default())/g' examples/chronos_slate.rs
sed -i 's/bind_group_layouts: &\[&bind_group_layout\],/bind_group_layouts: \&\[\&bind_group_layout\], immediate_size: 0,/g' examples/chronos_slate.rs
sed -i 's/wgpu::MaintainBase::Wait/wgpu::Maintain::Wait/g' examples/chronos_slate.rs
sed -i 's/app.global::<ChronosBus>()/app.global::<crate::ChronosBus>()/g' examples/chronos_slate.rs
