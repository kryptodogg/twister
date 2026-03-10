sed -i 's/wgpu::Maintain::wait()/wgpu::Maintain::Wait/g' examples/chronos_slate.rs
sed -i 's/let app_weak = app.as_weak();/let app_weak = app.as_weak();\n    let render_app_weak = app.as_weak();/g' examples/chronos_slate.rs
sed -i 's/if let Some(app) = app_weak.upgrade()/if let Some(app) = render_app_weak.upgrade()/g' examples/chronos_slate.rs
