// tests/mesh_shaders_integration.rs
// Integration tests for MeshShaderPipeline with adaptive 28-level LOD

#[cfg(test)]
mod mesh_shader_tests {
    use twister::visualization::mesh_shaders::{MeshLodLevel, MeshShaderPipeline};

    /// Test 1: LOD level creation and count
    #[test]
    fn test_lod_level_creation() {
        let lod_levels = MeshShaderPipeline::create_lod_levels();

        assert_eq!(
            lod_levels.len(),
            28,
            "Must have exactly 28 LOD levels (0-27)"
        );

        // Verify level numbers are sequential
        for (idx, level) in lod_levels.iter().enumerate() {
            assert_eq!(
                level.level, idx as u32,
                "LOD level {} should have level number {}",
                idx, idx
            );
        }
    }

    /// Test 2: LOD screen coverage thresholds
    #[test]
    fn test_lod_screen_coverage_thresholds() {
        let lod_levels = MeshShaderPipeline::create_lod_levels();

        // Verify coverage thresholds match specification
        // Levels 0-6: 2048px
        for idx in 0..7 {
            assert_eq!(
                lod_levels[idx].screen_coverage_pixels, 2048,
                "LOD level {} should have 2048px coverage",
                idx
            );
        }

        // Levels 7-13: 512px
        for idx in 7..14 {
            assert_eq!(
                lod_levels[idx].screen_coverage_pixels, 512,
                "LOD level {} should have 512px coverage",
                idx
            );
        }

        // Levels 14-20: 128px
        for idx in 14..21 {
            assert_eq!(
                lod_levels[idx].screen_coverage_pixels, 128,
                "LOD level {} should have 128px coverage",
                idx
            );
        }

        // Levels 21-27: 64px
        for idx in 21..28 {
            assert_eq!(
                lod_levels[idx].screen_coverage_pixels, 64,
                "LOD level {} should have 64px coverage",
                idx
            );
        }
    }

    /// Test 3: LOD vertex count decreases with LOD level
    #[test]
    fn test_lod_vertex_count_decreasing() {
        let lod_levels = MeshShaderPipeline::create_lod_levels();

        let mut prev_count = u32::MAX;
        for (idx, lod) in lod_levels.iter().enumerate() {
            assert!(
                lod.vertex_count <= prev_count,
                "Vertex count must decrease or stay same: level {} has {} vs previous {}",
                idx,
                lod.vertex_count,
                prev_count
            );
            prev_count = lod.vertex_count;
        }
    }

    /// Test 4: LOD triangle count decreases with LOD level
    #[test]
    fn test_lod_triangle_count_decreasing() {
        let lod_levels = MeshShaderPipeline::create_lod_levels();

        let mut prev_count = u32::MAX;
        for (idx, lod) in lod_levels.iter().enumerate() {
            assert!(
                lod.triangle_count <= prev_count,
                "Triangle count must decrease or stay same: level {} has {} vs previous {}",
                idx,
                lod.triangle_count,
                prev_count
            );
            prev_count = lod.triangle_count;
        }
    }

    /// Test 5: LOD high-detail (level 0) has high geometry
    #[test]
    fn test_lod_level_0_high_detail() {
        let lod_levels = MeshShaderPipeline::create_lod_levels();
        let level_0 = &lod_levels[0];

        // Level 0 should be high detail (aim for 1024 vertices)
        assert!(
            level_0.vertex_count >= 512,
            "Level 0 should have at least 512 vertices, got {}",
            level_0.vertex_count
        );
        assert!(
            level_0.screen_coverage_pixels >= 2048,
            "Level 0 should have coverage >= 2048px"
        );
    }

    /// Test 6: LOD lowest-detail (level 27) has low geometry
    #[test]
    fn test_lod_level_27_low_detail() {
        let lod_levels = MeshShaderPipeline::create_lod_levels();
        let level_27 = &lod_levels[27];

        // Level 27 should be low detail (aim for 64 vertices)
        assert!(
            level_27.vertex_count <= 128,
            "Level 27 should have at most 128 vertices, got {}",
            level_27.vertex_count
        );
        assert!(
            level_27.screen_coverage_pixels <= 64,
            "Level 27 should have coverage <= 64px"
        );
    }

    /// Test 7: Triangle-to-vertex ratio is reasonable (not degenerate)
    #[test]
    fn test_lod_triangle_to_vertex_ratio() {
        let lod_levels = MeshShaderPipeline::create_lod_levels();

        for (idx, lod) in lod_levels.iter().enumerate() {
            // For a sphere-like mesh: triangles ≈ vertices/2 to vertices
            // Allow some margin for efficiency
            if lod.vertex_count > 0 {
                let ratio = lod.triangle_count as f32 / lod.vertex_count as f32;
                assert!(
                    ratio > 0.1 && ratio < 2.0,
                    "Level {} has suspicious ratio: {} triangles / {} vertices = {}",
                    idx,
                    lod.triangle_count,
                    lod.vertex_count,
                    ratio
                );
            }
        }
    }

    /// Test 8: Shader module loading (mock test structure)
    #[test]
    fn test_shader_modules_defined() {
        // This test verifies that the shader source constants are non-empty
        // Actual compilation happens during cargo build

        let task_shader_src = include_str!("../src/visualization/shaders/mesh_lod_task.wgsl");
        let mesh_shader_src = include_str!("../src/visualization/shaders/mesh_lod_mesh.wgsl");
        let fragment_shader_src =
            include_str!("../src/visualization/shaders/mesh_lod_fragment.wgsl");

        assert!(
            !task_shader_src.is_empty(),
            "Task shader source must not be empty"
        );
        assert!(
            !mesh_shader_src.is_empty(),
            "Mesh shader source must not be empty"
        );
        assert!(
            !fragment_shader_src.is_empty(),
            "Fragment shader source must not be empty"
        );

        // Verify shader keywords are present
        assert!(
            task_shader_src.contains("@compute"),
            "Task shader must have @compute entry"
        );
        assert!(
            mesh_shader_src.contains("@vertex"),
            "Mesh shader must have @vertex entry"
        );
        assert!(
            fragment_shader_src.contains("@fragment"),
            "Fragment shader must have @fragment entry"
        );
    }

    /// Test 9: Heat map tonemap coverage (blue → red → yellow → white)
    #[test]
    fn test_heat_map_tonemap_function_present() {
        let fragment_shader_src =
            include_str!("../src/visualization/shaders/mesh_lod_fragment.wgsl");

        // Verify tonemap function is defined
        assert!(
            fragment_shader_src.contains("fn tonemap_intensity"),
            "Fragment shader must define tonemap_intensity function"
        );

        // Verify heat map color coverage
        assert!(
            fragment_shader_src.contains("0.33"),
            "Heat map should have transition at 0.33"
        );
        assert!(
            fragment_shader_src.contains("0.67"),
            "Heat map should have transition at 0.67"
        );
    }

    /// Test 10: Multiple instances can have different LOD levels simultaneously
    #[test]
    fn test_multiple_instances_different_lods() {
        let lod_levels = MeshShaderPipeline::create_lod_levels();

        // Simulate 4 instances with different projected screen sizes
        let screen_sizes = vec![3000.0, 1000.0, 300.0, 100.0]; // pixels
        let mut selected_lods = Vec::new();

        for screen_size in screen_sizes {
            let lod = MeshShaderPipeline::select_lod_for_screen_size(screen_size, &lod_levels);
            selected_lods.push(lod);
        }

        // Verify we get different LODs for different screen sizes
        assert_eq!(selected_lods[0], 0, "3000px should select level 0");
        assert!(selected_lods[1] > 0, "1000px should select level > 0");
        assert!(
            selected_lods[2] > selected_lods[1],
            "300px should select higher LOD than 1000px"
        );
        assert!(
            selected_lods[3] > selected_lods[2],
            "100px should select highest LOD"
        );
    }

    /// Test 11: LOD selection boundary conditions
    #[test]
    fn test_lod_selection_boundary_conditions() {
        let lod_levels = MeshShaderPipeline::create_lod_levels();

        // Test exact boundary values
        let lod_at_2048 = MeshShaderPipeline::select_lod_for_screen_size(2048.0, &lod_levels);
        let lod_at_512 = MeshShaderPipeline::select_lod_for_screen_size(512.0, &lod_levels);
        let lod_at_128 = MeshShaderPipeline::select_lod_for_screen_size(128.0, &lod_levels);
        let lod_at_64 = MeshShaderPipeline::select_lod_for_screen_size(64.0, &lod_levels);

        assert!(
            lod_at_2048 < 7,
            "2048px should be in high-detail range (0-6)"
        );
        assert!(
            lod_at_512 >= 7 && lod_at_512 < 14,
            "512px should be in medium range (7-13)"
        );
        assert!(
            lod_at_128 >= 14 && lod_at_128 < 21,
            "128px should be in low range (14-20)"
        );
        assert!(lod_at_64 >= 21, "64px should be in lowest range (21-27)");
    }

    /// Test 12: Verify LOD metadata is consistent
    #[test]
    fn test_lod_metadata_consistency() {
        let lod_levels = MeshShaderPipeline::create_lod_levels();

        for level in &lod_levels {
            // Vertex and triangle counts should be positive
            assert!(
                level.vertex_count > 0,
                "Level {} has 0 vertices",
                level.level
            );
            assert!(
                level.triangle_count > 0,
                "Level {} has 0 triangles",
                level.level
            );

            // Screen coverage should be in reasonable range
            assert!(
                level.screen_coverage_pixels >= 64,
                "Coverage < 64px for level {}",
                level.level
            );
            assert!(
                level.screen_coverage_pixels <= 2048,
                "Coverage > 2048px for level {}",
                level.level
            );
        }
    }
}
