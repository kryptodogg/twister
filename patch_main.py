import re

with open("src/main.rs", "r") as f:
    code = f.read()

# remove redundant imports
code = re.sub(r'use crate::particle_system::renderer::ParticleRenderer;\n', '', code)
code = re.sub(r'use crate::particle_system::frustum_culler::FrustumCuller;\n', '', code)
code = re.sub(r'use crate::particle_system::streaming::ParticleStreamLoader;\n', '', code)

# Fix duplicate instantiation
# We see two definitions.
# Let's completely replace the block between "let particle_renderer = " and "state.running.store("
block_start = "    let particle_renderer ="
block_end = "    state.running.store(true, Ordering::Relaxed);"

replacement = """    let particle_renderer = crate::particle_system::renderer::ParticleRenderer::new(
        gpu_shared.clone(),
        10_000_000,
        wgpu::TextureFormat::Rgba8Unorm,
    );
    let frustum_culler = crate::particle_system::frustum_culler::FrustumCuller::new(gpu_shared.clone(), 10_000_000);
    let particle_streamer = std::sync::Arc::new(crate::particle_system::streaming::ParticleStreamLoader::new());

    let now_ms = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64;
    let _ = tokio::spawn({
        let s = particle_streamer.clone();
        async move { s.load_window(now_ms - 8_380_800_000, now_ms, 1_000_000).await; }
    });

    state.running.store(true, std::sync::atomic::Ordering::Relaxed);"""

# I'll just use regex to replace anything that looks like ParticleSystem init block
pattern = r"\s*let particle_renderer = .*?state\.running\.store\(true, (?:std::sync::atomic::)?Ordering::Relaxed\);"

code = re.sub(pattern, "\n" + replacement, code, flags=re.DOTALL)

with open("src/main.rs", "w") as f:
    f.write(code)
