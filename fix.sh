sed -i 's/\.request_device(\.request_device(&wgpu::DeviceDescriptor::default())(&wgpu::DeviceDescriptor::default()))/.request_device(\&wgpu::DeviceDescriptor::default(), None)/g' examples/chronos_slate.rs
sed -i 's/wgpu::Maintain::Wait/wgpu::Maintain::wait()/g' examples/chronos_slate.rs
