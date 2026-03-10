// src/ml/vbuffer_render.rs
// Visibility Buffer Render Pass (Geometry Pass)
//
// Renders particle IDs (u32 integers) to a full-screen texture.
// No color/material evaluation yet—just identify which particle is at each pixel.
// Subsequent Wyrdfall resolve pass will do the expensive Mamba lookups.

use wgpu::{Buffer, Device, Queue, Texture, TextureView, RenderPipeline};

/// V-Buffer render target (stores particle IDs)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct VBufferPixel {
    /// Particle ID (index into FieldParticleBuffer)
    pub particle_id: u32,
    /// Screen-space depth (for future reorder-based rendering)
    pub depth: f32,
}

/// Render configuration for V-Buffer pass
#[derive(Clone, Debug)]
pub struct VBufferRenderConfig {
    /// Render resolution (typically 1080p for FSR 3.1 upscaling)
    pub width: u32,
    pub height: u32,
    /// Near/far clip planes
    pub near_plane: f32,
    pub far_plane: f32,
}

impl Default for VBufferRenderConfig {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
            near_plane: 0.1,
            far_plane: 1000.0,
        }
    }
}

/// V-Buffer render state
pub struct VBufferRenderer {
    /// Configuration
    pub config: VBufferRenderConfig,
    /// Output texture (RGBA32Uint for particle IDs + depth)
    pub vbuffer_texture: Option<Texture>,
    pub vbuffer_view: Option<TextureView>,
    /// Depth texture for Z-testing
    pub depth_texture: Option<Texture>,
    pub depth_view: Option<TextureView>,
    /// Render pipeline (cached for reuse)
    pub render_pipeline: Option<RenderPipeline>,
    /// Frame counter
    pub frame_count: u32,
}

impl VBufferRenderer {
    /// Create new V-Buffer renderer
    pub fn new(config: VBufferRenderConfig) -> Self {
        Self {
            config,
            vbuffer_texture: None,
            vbuffer_view: None,
            depth_texture: None,
            depth_view: None,
            render_pipeline: None,
            frame_count: 0,
        }
    }

    /// Initialize GPU textures (call once after device available)
    pub fn initialize(&mut self, device: &Device) {
        // V-Buffer texture: RGBA32Uint (particle_id in R, depth in G, unused in B,A)
        let vbuffer_descriptor = wgpu::TextureDescriptor {
            label: Some("VBuffer"),
            size: wgpu::Extent3d {
                width: self.config.width,
                height: self.config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba32Uint,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[wgpu::TextureFormat::Rgba32Uint],
        };
        let vbuffer_tex = device.create_texture(&vbuffer_descriptor);
        let vbuffer_view = vbuffer_tex.create_view(&wgpu::TextureViewDescriptor::default());

        // Depth texture: Depth32Float
        let depth_descriptor = wgpu::TextureDescriptor {
            label: Some("VBufferDepth"),
            size: wgpu::Extent3d {
                width: self.config.width,
                height: self.config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[wgpu::TextureFormat::Depth32Float],
        };
        let depth_tex = device.create_texture(&depth_descriptor);
        let depth_view = depth_tex.create_view(&wgpu::TextureViewDescriptor::default());

        self.vbuffer_texture = Some(vbuffer_tex);
        self.vbuffer_view = Some(vbuffer_view);
        self.depth_texture = Some(depth_tex);
        self.depth_view = Some(depth_view);
    }

    /// Get V-Buffer texture view for rendering
    pub fn vbuffer_view(&self) -> Option<&TextureView> {
        self.vbuffer_view.as_ref()
    }

    /// Get depth texture view for depth testing
    pub fn depth_view(&self) -> Option<&TextureView> {
        self.depth_view.as_ref()
    }

    /// Begin V-Buffer render pass
    /// Returns encoder for wgpu command recording
    pub fn begin_render_pass<'a>(
        &mut self,
        encoder: &'a mut wgpu::CommandEncoder,
    ) -> Option<wgpu::RenderPass<'a>> {
        let vbuffer_view = self.vbuffer_view.as_ref()?;
        let depth_view = self.depth_view.as_ref()?;

        let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("VBuffer Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: vbuffer_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: 0.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });

        Some(render_pass)
    }

    /// Increment frame counter (for temporal coherence in Wyrdfall)
    pub fn next_frame(&mut self) {
        self.frame_count += 1;
    }

    /// Statistics about the V-Buffer
    pub fn stats(&self) -> VBufferStats {
        VBufferStats {
            resolution_pixels: self.config.width * self.config.height,
            frames_rendered: self.frame_count,
        }
    }
}

/// Statistics snapshot
#[derive(Clone, Debug)]
pub struct VBufferStats {
    pub resolution_pixels: u32,
    pub frames_rendered: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vbuffer_creation() {
        let config = VBufferRenderConfig::default();
        let renderer = VBufferRenderer::new(config);

        assert_eq!(renderer.config.width, 1920);
        assert_eq!(renderer.config.height, 1080);
        assert_eq!(renderer.frame_count, 0);
    }

    #[test]
    fn test_vbuffer_frame_increment() {
        let mut renderer = VBufferRenderer::new(VBufferRenderConfig::default());
        renderer.next_frame();
        renderer.next_frame();

        assert_eq!(renderer.frame_count, 2);
    }

    #[test]
    fn test_vbuffer_stats() {
        let renderer = VBufferRenderer::new(VBufferRenderConfig::default());
        let stats = renderer.stats();

        assert_eq!(stats.resolution_pixels, 1920 * 1080);
    }

    #[test]
    fn test_custom_resolution() {
        let config = VBufferRenderConfig {
            width: 2560,
            height: 1440,
            ..Default::default()
        };
        let renderer = VBufferRenderer::new(config);

        assert_eq!(renderer.stats().resolution_pixels, 2560 * 1440);
    }
}
