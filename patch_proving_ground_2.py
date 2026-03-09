with open('examples/megalights_proving_ground.rs', 'r') as f:
    content = f.read()

content = content.replace(
"""    let (device, queue) = match adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("Megalights Device"),
            required_features,
            required_limits: wgpu::Limits::default(),
            memory_hints: Default::default(),
        },
        None,
    ).await {""",
"""    let (device, queue) = match adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("Megalights Device"),
            required_features,
            required_limits: wgpu::Limits::default(),
            memory_hints: Default::default(),
        },
        None,
    ).await {""")

# We actually need to fix `request_device` which only takes 1 argument in this version and fix `DeviceDescriptor`
new_device_request = """    let (device, queue) = match adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("Megalights Device"),
            required_features,
            required_limits: wgpu::Limits::default(),
            memory_hints: Default::default(),
        },
        None,
    ).await {"""

new_device_request_fixed = """    let (device, queue) = match adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("Megalights Device"),
            required_features,
            required_limits: wgpu::Limits::default(),
            memory_hints: Default::default(),
        }
    ).await {"""
content = content.replace(new_device_request, new_device_request_fixed)

# Wait, `wgpu::DeviceDescriptor` might also need `experimental_features: Default::default()` and `trace: None` in wgpu 28.0.0? Yes.
# Or wait, no `experimental_features` was missing in example, let's fix it inside the script:
content = content.replace("memory_hints: Default::default(),", "memory_hints: Default::default(),\n            ..Default::default()")

with open('examples/megalights_proving_ground.rs', 'w') as f:
    f.write(content)
