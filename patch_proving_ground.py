with open('examples/megalights_proving_ground.rs', 'r') as f:
    content = f.read()

content = content.replace("wgpu::Instance::new(wgpu::InstanceDescriptor", "wgpu::Instance::new(&wgpu::InstanceDescriptor")
content = content.replace(".await.ok_or", ".await.ok().ok_or")
content = content.replace("wgpu::Features::RAY_QUERY | wgpu::Features::RAY_TRACING_ACCELERATION_STRUCTURE", "wgpu::Features::empty()") # Just fallback to empty since they are experimental and not strictly defined in stable wgpu
content = content.replace(
"""        &wgpu::DeviceDescriptor {
            label: Some("Megalights Device"),
            required_features,
            required_limits: wgpu::Limits::default(),
        }""",
"""        &wgpu::DeviceDescriptor {
            label: Some("Megalights Device"),
            required_features,
            required_limits: wgpu::Limits::default(),
            memory_hints: Default::default(),
        }""")

with open('examples/megalights_proving_ground.rs', 'w') as f:
    f.write(content)
