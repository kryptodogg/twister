sed -i 's/\.request_device(&wgpu::DeviceDescriptor::default(), None)/.request_device(\&wgpu::DeviceDescriptor::default())/g' examples/chronos_slate.rs
sed -i 's/push_constant_ranges: &\[\],/ /g' examples/chronos_slate.rs
sed -i 's/multiview: None,/multiview_mask: None,/g' examples/chronos_slate.rs
sed -i 's/view: &self.texture_view,/view: \&self.texture_view, depth_slice: None,/g' examples/chronos_slate.rs
sed -i 's/occlusion_query_set: None,/occlusion_query_set: None, multiview_mask: None,/g' examples/chronos_slate.rs
sed -i 's/bind_group_layouts: &\[&camera_bind_group_layout\],/bind_group_layouts: \&\[\&camera_bind_group_layout\], immediate_size: 0,/g' examples/chronos_slate.rs
sed -i 's/bind_group_layouts: &\[&camera_bind_group_layout, &particle_bind_group_layout\],/bind_group_layouts: \&\[\&camera_bind_group_layout, \&particle_bind_group_layout\], immediate_size: 0,/g' examples/chronos_slate.rs
sed -i 's/wgpu::Maintain::Wait/wgpu::Maintain::wait()/g' examples/chronos_slate.rs
