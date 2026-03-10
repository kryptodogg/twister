sed -i 's/\.request_device(\.request_device(&wgpu::DeviceDescriptor::default())(&wgpu::DeviceDescriptor::default()))/.request_device(\&wgpu::DeviceDescriptor::default(), None)/g' examples/chronos_slate.rs
sed -i 's/\.request_device(\.request_device(\&wgpu::DeviceDescriptor::default(), None), None)/.request_device(\&wgpu::DeviceDescriptor::default(), None)/g' examples/chronos_slate.rs
sed -i 's/wgpu::Maintain::wait()/wgpu::MaintainBase::Wait/g' examples/chronos_slate.rs
sed -i 's/import { ChronosSlate, ChronosBus }/import { ChronosBus, ChronosSlate }/g' examples/chronos_slate.rs
