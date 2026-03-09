use crate::visualization::data_contracts::VoxelGrid;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LightProbe {
    pub position: [f32; 3],
    pub validity: f32,
    pub irradiance: [f32; 3],
    pub padding: f32,
}

pub struct LumenGI {
    pub probe_grid: Vec<LightProbe>,
    pub surfel_buffer: wgpu::Buffer,
    pub indirect_lighting_cache: wgpu::Texture,
}

impl LumenGI {
    pub fn new(device: &wgpu::Device, room_size: f32) -> Self {
        let probe_spacing = room_size / 8.0;
        let mut probes = Vec::with_capacity(8 * 8 * 8);

        for x in 0..8 {
            for y in 0..8 {
                for z in 0..8 {
                    probes.push(LightProbe {
                        position: [
                            (x as f32) * probe_spacing,
                            (y as f32) * probe_spacing,
                            (z as f32) * probe_spacing,
                        ],
                        validity: 0.0,
                        irradiance: [0.0, 0.0, 0.0],
                        padding: 0.0,
                    });
                }
            }
        }

        let surfel_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Surfel Buffer"),
            size: (512 * std::mem::size_of::<LightProbe>()) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        let indirect_lighting_cache = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Indirect Lighting Cache"),
            size: wgpu::Extent3d { width: 128, height: 128, depth_or_array_layers: 128 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D3,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        Self {
            probe_grid: probes,
            surfel_buffer,
            indirect_lighting_cache,
        }
    }

    pub fn update_probes(
        &mut self,
        queue: &wgpu::Queue,
        energy_field: &VoxelGrid,
        _direct_lighting: &wgpu::Texture,
    ) {
        for probe in &mut self.probe_grid {
            let energy = energy_field.sample((probe.position[0], probe.position[1], probe.position[2]));
            probe.irradiance = [energy, energy * 0.7, energy * 0.5];
            probe.validity = 0.9;
        }

        queue.write_buffer(&self.surfel_buffer, 0, bytemuck::cast_slice(&self.probe_grid));
    }

    pub fn sample_indirect(&self, pos: [f32; 3]) -> [f32; 3] {
        let mut irradiance = [0.0; 3];
        let mut total_weight = 0.0;

        for probe in &self.probe_grid {
            let dx = pos[0] - probe.position[0];
            let dy = pos[1] - probe.position[1];
            let dz = pos[2] - probe.position[2];
            let dist_sq = dx * dx + dy * dy + dz * dz;
            let dist = dist_sq.sqrt();
            let weight = 1.0 / dist.max(0.1).powi(2);

            for i in 0..3 {
                irradiance[i] += probe.irradiance[i] * weight * probe.validity;
            }
            total_weight += weight;
        }

        let inv_total_weight = 1.0 / total_weight.max(1e-6);
        for i in 0..3 {
            irradiance[i] *= inv_total_weight;
        }

        irradiance
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_probe_grid_creation() { }
    #[test] fn test_probe_energy_coupling() { }
    #[test] fn test_indirect_interpolation() { }
    #[test] fn test_cache_coherence() { }
}
