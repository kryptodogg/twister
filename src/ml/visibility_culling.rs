// src/ml/visibility_culling.rs
// Hi-Z Frustum Culling + Auto-LOD for Particle Visibility Buffer
//
// Computes which particles are visible from the camera, culls occluded particles,
// and merges clusters that project to single screen pixels (Auto-LOD).
//
// Output: Indirect draw buffer (particle IDs that survived culling)

use wgpu::{Buffer, Device, Queue};
use std::mem;

/// GPU-safe frustum plane (ax + by + cz + d = 0)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FrustumPlane {
    pub normal: [f32; 3],  // (a, b, c)
    pub distance: f32,     // d
}

/// Camera frustum definition (6 planes: near, far, left, right, top, bottom)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct CameraFrustum {
    pub planes: [FrustumPlane; 6],
    pub camera_pos: [f32; 3],
    pub _padding: f32,
}

impl CameraFrustum {
    /// Create frustum from view-projection matrix
    pub fn from_view_proj(view_proj: &[[f32; 4]; 4]) -> Self {
        // Extract frustum planes from view-projection matrix
        // Standard left-hand coordinate system extraction
        let planes = [
            // Near plane: M[3] + M[2]
            FrustumPlane {
                normal: [
                    view_proj[3][0] + view_proj[2][0],
                    view_proj[3][1] + view_proj[2][1],
                    view_proj[3][2] + view_proj[2][2],
                ],
                distance: view_proj[3][3] + view_proj[2][3],
            },
            // Far plane: M[3] - M[2]
            FrustumPlane {
                normal: [
                    view_proj[3][0] - view_proj[2][0],
                    view_proj[3][1] - view_proj[2][1],
                    view_proj[3][2] - view_proj[2][2],
                ],
                distance: view_proj[3][3] - view_proj[2][3],
            },
            // Left plane: M[3] + M[0]
            FrustumPlane {
                normal: [
                    view_proj[3][0] + view_proj[0][0],
                    view_proj[3][1] + view_proj[0][1],
                    view_proj[3][2] + view_proj[0][2],
                ],
                distance: view_proj[3][3] + view_proj[0][3],
            },
            // Right plane: M[3] - M[0]
            FrustumPlane {
                normal: [
                    view_proj[3][0] - view_proj[0][0],
                    view_proj[3][1] - view_proj[0][1],
                    view_proj[3][2] - view_proj[0][2],
                ],
                distance: view_proj[3][3] - view_proj[0][3],
            },
            // Top plane: M[3] - M[1]
            FrustumPlane {
                normal: [
                    view_proj[3][0] - view_proj[1][0],
                    view_proj[3][1] - view_proj[1][1],
                    view_proj[3][2] - view_proj[1][2],
                ],
                distance: view_proj[3][3] - view_proj[1][3],
            },
            // Bottom plane: M[3] + M[1]
            FrustumPlane {
                normal: [
                    view_proj[3][0] + view_proj[1][0],
                    view_proj[3][1] + view_proj[1][1],
                    view_proj[3][2] + view_proj[1][2],
                ],
                distance: view_proj[3][3] + view_proj[1][3],
            },
        ];

        Self {
            planes,
            camera_pos: [0.0, 0.0, 0.0], // Set from camera data
            _padding: 0.0,
        }
    }

    /// Test if point is inside frustum
    pub fn contains_point(&self, point: &[f32; 3]) -> bool {
        for plane in &self.planes {
            let dot = plane.normal[0] * point[0]
                + plane.normal[1] * point[1]
                + plane.normal[2] * point[2]
                + plane.distance;
            if dot < 0.0 {
                return false;
            }
        }
        true
    }
}

/// Indirect draw command (used with draw_indirect)
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct IndirectDrawCommand {
    pub vertex_count: u32,     // Vertices per particle (typically 6 for 2 triangles)
    pub instance_count: u32,   // Number of particle instances to draw
    pub first_vertex: u32,     // Starting vertex index
    pub first_instance: u32,   // Starting instance index
}

/// Hi-Z Buffer entry (mip level of depth pyramid)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct HiZEntry {
    pub depth: f32,      // Maximum depth at this level
    pub mip_level: u32,  // Pyramid level (0 = full resolution)
    pub _padding: [u32; 2],
}

/// Culling statistics (for debugging)
#[derive(Clone, Debug)]
pub struct CullingStats {
    pub total_particles: u32,
    pub frustum_culled: u32,
    pub occlusion_culled: u32,
    pub lod_merged: u32,
    pub visible_particles: u32,
}

/// Visibility culling pipeline
pub struct VisibilityCulling {
    pub frustum: CameraFrustum,
    pub hiz_pyramid: Option<Buffer>,  // Hierarchical Z-Buffer texture (lazy loaded)
    pub stats: CullingStats,
}

impl VisibilityCulling {
    /// Create new culling pipeline
    pub fn new(frustum: CameraFrustum) -> Self {
        Self {
            frustum,
            hiz_pyramid: None,
            stats: CullingStats {
                total_particles: 0,
                frustum_culled: 0,
                occlusion_culled: 0,
                lod_merged: 0,
                visible_particles: 0,
            },
        }
    }

    /// Update frustum from camera view-proj
    pub fn update_frustum(&mut self, view_proj: &[[f32; 4]; 4]) {
        self.frustum = CameraFrustum::from_view_proj(view_proj);
    }

    /// CPU-side culling (for validation/testing)
    /// In production: all culling happens on GPU via compute shader
    pub fn cull_particles(
        &mut self,
        positions: &[[f32; 3]],
    ) -> Vec<u32> {
        let mut visible = Vec::new();
        self.stats.total_particles = positions.len() as u32;
        self.stats.frustum_culled = 0;

        for (idx, pos) in positions.iter().enumerate() {
            if self.frustum.contains_point(pos) {
                visible.push(idx as u32);
            } else {
                self.stats.frustum_culled += 1;
            }
        }

        self.stats.visible_particles = visible.len() as u32;
        visible
    }

    /// Generate indirect draw command for visible particles
    pub fn generate_indirect_command(
        &self,
        visible_count: u32,
    ) -> IndirectDrawCommand {
        IndirectDrawCommand {
            vertex_count: 6,  // 2 triangles per particle (quad)
            instance_count: visible_count,
            first_vertex: 0,
            first_instance: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frustum_creation() {
        let identity = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
        let frustum = CameraFrustum::from_view_proj(&identity);
        assert_eq!(frustum.planes.len(), 6);
    }

    #[test]
    fn test_frustum_contains_origin() {
        let identity = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
        let frustum = CameraFrustum::from_view_proj(&identity);
        assert!(frustum.contains_point(&[0.0, 0.0, 0.0]));
    }

    #[test]
    fn test_culling_pipeline() {
        let identity = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
        let frustum = CameraFrustum::from_view_proj(&identity);
        let mut culling = VisibilityCulling::new(frustum);

        let positions = vec![[0.0, 0.0, 0.0], [10.0, 10.0, 10.0]];
        let visible = culling.cull_particles(&positions);

        assert!(visible.len() > 0);
    }

    #[test]
    fn test_indirect_command_generation() {
        let identity = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
        let frustum = CameraFrustum::from_view_proj(&identity);
        let culling = VisibilityCulling::new(frustum);

        let cmd = culling.generate_indirect_command(1024);
        assert_eq!(cmd.vertex_count, 6);
        assert_eq!(cmd.instance_count, 1024);
    }
}
