import re

with open('src/gpu.rs', 'r') as f:
    content = f.read()

# Fix PipelineLayoutDescriptor
content = content.replace("""        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("signal_processor_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[wgpu::PushConstantRange {
                stages: wgpu::ShaderStages::COMPUTE,
                range: 0..std::mem::size_of::<VBufferPushConst>() as u32,
            }],
        });""", """        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("signal_processor_layout"),
            bind_group_layouts: &[&bind_group_layout],
            immediate_size: std::mem::size_of::<VBufferPushConst>() as u32,
        });""")

# Fix entry_point
content = content.replace('entry_point: "main",', 'entry_point: Some("main"),')

# Fix cache
content = content.replace('compilation_options: Default::default(),', 'compilation_options: Default::default(),\n            cache: None,')

# Fix set_push_constants
content = content.replace('cpass.set_push_constants(0, pc_bytes);', 'cpass.set_immediate_data(pc_bytes);')

with open('src/gpu.rs', 'w') as f:
    f.write(content)
