sed -i 's/let render_app_weak = app.as_weak();/let render_app_weak = app.as_weak();\n    let handle = app.as_weak();/g' examples/chronos_slate.rs
sed -i 's/if let Some(app) = render_app_weak.upgrade()/let app_weak_clone = render_app_weak.clone();\n            if app_weak_clone.upgrade().is_some()/g' examples/chronos_slate.rs
sed -i 's/app.set_viewport_texture(Image::from_rgba8(pixels));/if let Some(app) = app_weak_clone.upgrade() { app.set_viewport_texture(Image::from_rgba8(pixels)); }/g' examples/chronos_slate.rs
