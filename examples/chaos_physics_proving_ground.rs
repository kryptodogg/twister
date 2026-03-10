// A standalone proving ground script to visually verify RF-matter coupling physics

slint::slint! {
    export component PhysicsVisualizer inherits Window {
        width: 800px;
        height: 600px;
        title: "Chaos Physics Proving Ground: 2.4 GHz Heterodyne Interaction";

        in-out property <image> rf_field_image;
        in-out property <string> status_text: "Running Simulation...";

        VerticalLayout {
            padding: 20px;
            spacing: 15px;

            Text {
                text: "Track VI.1 Proving Ground: RF-Matter Coupling";
                font-size: 24px;
                font-weight: 700;
                horizontal-alignment: center;
            }

            Rectangle {
                background: #111111;
                border-color: #444444;
                border-width: 2px;
                border-radius: 4px;

                Image {
                    source: root.rf_field_image;
                    width: 100%;
                    height: 100%;
                    image-fit: contain;
                }
            }

            Text {
                text: root.status_text;
                font-size: 16px;
                color: #888888;
                horizontal-alignment: center;
            }
        }
    }
}

use num_complex::Complex;
use slint::{Image, Rgba8Pixel, SharedPixelBuffer};
use std::sync::Arc;
use twister::resonance::body_interaction_model::HumanBody;
use twister::resonance::heterodyne_mixer::HeterodyneMixer;
use twister::resonance::material_absorption::MaterialGrid;
use twister::resonance::rf_propagation::RFWavePropagation;
use twister::resonance::voxel_grid::VoxelGrid;
use twister::visualization::data_contracts::{PoseFrame, RoomGeometry};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting Chaos Physics Proving Ground...");

    // 1. Setup Simulation Environment
    let grid_size = 64; // High res for visualization
    let voxel_size_m = 0.05; // 5cm voxels for a ~3.2m room slice
    let primary_freq_hz = 2.4e9; // 2.4 GHz

    // Create the Slint UI
    let ui = PhysicsVisualizer::new()?;

    // Setup initial geometry
    let room = RoomGeometry {
        min_bound: (0.0, 0.0, 0.0),
        max_bound: (3.2, 3.2, 3.2),
    };

    // Create materials (air everywhere to start)
    let mut material_grid = MaterialGrid::from_room_geometry(&room, primary_freq_hz, voxel_size_m);

    // Setup a human body in the middle
    let mut pose = PoseFrame::default();
    pose.keypoints.push((1.6, 1.6, 1.6)); // Torso center
    let human = HumanBody::from_pose(&pose, grid_size, voxel_size_m);

    // Merge human into materials
    for x in 0..material_grid.grid.dimensions.0 {
        for y in 0..material_grid.grid.dimensions.1 {
            for z in 0..material_grid.grid.dimensions.2 {
                if human.voxel_map.get(x, y, z) > 0.5 {
                    let mut tissue = twister::resonance::material_absorption::Material::default();
                    tissue.name = "Tissue".to_string();
                    tissue.permittivity = 50.0;
                    tissue.conductivity = 1.0;
                    material_grid.grid.set(x, y, z, tissue);
                }
            }
        }
    }

    // 2. Setup RF Solver
    let mut rf_sim = RFWavePropagation::new(grid_size, primary_freq_hz);
    rf_sim.grid.voxel_size_m = voxel_size_m;
    rf_sim.grid.origin = room.min_bound;

    // Run solver with source on the left
    let source_pos = (0.5, 1.6, 1.6);
    println!("Solving Wave Equation...");
    rf_sim.solve_wave_equation(source_pos, 1.0, &material_grid.grid)?;

    // 3. Render cross-section to Slint Image
    println!("Rendering slice...");
    let dim_x = rf_sim.grid.dimensions.0;
    let dim_y = rf_sim.grid.dimensions.1;
    let z_slice = dim_y / 2; // Mid-plane slice

    let mut pixel_buffer = SharedPixelBuffer::<Rgba8Pixel>::new(dim_x as u32, dim_y as u32);

    for y in 0..dim_y {
        for x in 0..dim_x {
            // Get field at this voxel
            let complex_field = rf_sim.grid.get(x, y, z_slice);

            // Map magnitude to intensity
            let magnitude = complex_field.norm();
            let phase = complex_field.arg(); // -pi to pi

            // Check if it's human tissue
            let is_human = human.voxel_map.get(x, y, z_slice) > 0.5;

            // Map to RGB (Phase -> Hue, Magnitude -> Brightness)
            // Simplified: Red/Blue for phase, black for low magnitude
            let intensity = (magnitude * 255.0).min(255.0) as u8;

            let mut r = intensity;
            let mut g = (intensity as f32 * (phase.cos() * 0.5 + 0.5)) as u8;
            let mut b = (intensity as f32 * (phase.sin() * 0.5 + 0.5)) as u8;

            if is_human {
                // Overlay human outline
                r = r.saturating_add(100);
                g = g.saturating_add(50);
                b = b.saturating_add(50);
            }

            // Slint pixel buffer is y-down, x-right
            let idx = (y * dim_x + x) as usize;

            let mut pixels = pixel_buffer.make_mut_slice();
            if idx < pixels.len() {
                pixels[idx] = Rgba8Pixel { r, g, b, a: 255 };
            }
        }
    }

    ui.set_rf_field_image(Image::from_rgba8(pixel_buffer));
    ui.set_status_text("Simulation Complete: Notice the attenuation and scattering around the human body structure.".into());

    println!("Launching UI...");
    ui.run()?;

    Ok(())
}
