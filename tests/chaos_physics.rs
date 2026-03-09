use num_complex::Complex;
use std::error::Error;
use twister::physics::body_interaction_model::HumanBody;
use twister::physics::heterodyne_mixer::HeterodyneMixer;
use twister::physics::material_absorption::{Material, MaterialGrid};
use twister::physics::rf_propagation::RFWavePropagation;
use twister::physics::voxel_grid::VoxelGrid;
use twister::visualization::data_contracts::{PoseFrame, RoomGeometry};

#[test]
fn test_wave_equation_solver() {
    // Helmholtz verification
    let freq_hz = 2.4e9;
    let mut rf = RFWavePropagation::new(32, freq_hz);
    let mut mats = VoxelGrid::new(32);
    // Fill air
    for x in 0..32 {
        for y in 0..32 {
            for z in 0..32 {
                mats.set(x, y, z, Material::air());
            }
        }
    }

    rf.solve_wave_equation((1.6, 1.6, 1.6), 1.0, &mats).unwrap();
    let (mag, phase) = rf.field_at((1.6, 1.6, 1.6));
    assert!(mag > 0.0);
}

#[test]
fn test_phase_continuity() {
    // Phase gradient is continuous
    let mut rf = RFWavePropagation::new(32, 2.4e9);
    rf.grid.set(16, 16, 16, Complex::new(1.0, 0.0));
    let (m1, p1) = rf.field_at((1.6, 1.6, 1.6));
    let (m2, p2) = rf.field_at((1.61, 1.6, 1.6));
    assert!((p1 - p2).abs() < 1.0); // should be smooth
}

#[test]
fn test_energy_conservation() {
    // Ensure the SOR doesn't blow up to infinity
    let mut rf = RFWavePropagation::new(16, 2.4e9);
    let mut mats = VoxelGrid::new(16);
    for x in 0..16 {
        for y in 0..16 {
            for z in 0..16 {
                mats.set(x, y, z, Material::air());
            }
        }
    }

    rf.solve_wave_equation((0.8, 0.8, 0.8), 1.0, &mats).unwrap();
    let val = rf.grid.get(8, 8, 8).norm();
    assert!(val < 1e5); // Stable
}

#[test]
fn test_plane_wave_source() {
    let mut rf = RFWavePropagation::new(16, 2.4e9);
    let mats = VoxelGrid::new(16);
    rf.solve_wave_equation((0.0, 0.0, 0.0), 1.0, &mats).unwrap();
    assert_eq!(rf.grid.get(0, 0, 0), Complex::new(1.0, 0.0));
}

#[test]
fn test_high_frequency_accuracy() {
    let rf = RFWavePropagation::new(10, 2.4e9);
    assert!((rf.wavelength_m - 0.125).abs() < 0.001); // 2.4 GHz wavelength = 12.5cm
}

#[test]
fn test_drywall_absorption() {
    let freq = 2.4e9;
    let material = Material::drywall();
    let alpha = material.attenuation_coeff(freq);
    assert!(alpha > 0.1 && alpha < 5.0); // Moderate loss
}

#[test]
fn test_water_absorption() {
    let freq = 2.4e9;
    let material = Material::water();
    let alpha = material.attenuation_coeff(freq);
    assert!(alpha > 10.0); // High loss
}

#[test]
fn test_reflection_coeff_hardness() {
    let material = Material::concrete(); // 0.9 hardness
    let reflect = material.reflection_coeff(2.4e9);
    assert!((reflect - 0.9).abs() < 0.01);
}

#[test]
fn test_scattering_roughness() {
    let material = Material::drywall(); // 0.5 roughness
    assert_eq!(material.scattering_coeff(), 0.5);
}

#[test]
fn test_sideband_generation() {
    let mixer = HeterodyneMixer::new(2.4e9, 4e3);
    assert_eq!(mixer.heterodyne_freqs.len(), 3);
    assert_eq!(mixer.heterodyne_freqs[0], 2.4e9 - 4e3);
    assert_eq!(mixer.heterodyne_freqs[2], 2.4e9 + 4e3);
}

#[test]
fn test_phase_relationship() {
    let mixer = HeterodyneMixer::new(2.4e9, 4e3);
    let rf = Complex::new(1.0, 0.0);
    let mixed = mixer.mix_signals(rf, std::f32::consts::PI / 4.0);
    assert_eq!(mixed.len(), 3);
    // Lower and upper sidebands should have symmetric phase relationships
    assert_eq!(mixed[0].re, mixed[2].re);
}

#[test]
fn test_modulation_depth() {
    let mixer = HeterodyneMixer::new(2.4e9, 4e3);
    let eff = mixer.sideband_efficiency();
    assert!(eff < 1.0); // m < 1 for AM
}

#[test]
fn test_heterodyne_coupling() {
    let mixer = HeterodyneMixer::new(2.4e9, 4e3);
    let rf = Complex::new(1.0, 0.0);
    let mixed = mixer.mix_signals(rf, 0.0); // Audio mod 0 => cos(0) = 1, sin(0) = 0
    assert_eq!(mixed[0], Complex::new(0.5, 0.0)); // Lower
    assert_eq!(mixed[1], Complex::new(1.0, 0.0)); // Primary
    assert_eq!(mixed[2], Complex::new(0.5, 0.0)); // Upper
}

#[test]
fn test_body_voxelization() {
    let mut frame = PoseFrame::default();
    frame.keypoints.push((1.0, 1.0, 1.0));
    let body = HumanBody::from_pose(&frame, 32, 0.1);

    let (gx, gy, gz) = body.voxel_map.world_to_grid((1.0, 1.0, 1.0));
    let x = gx as usize;
    let y = gy as usize;
    let z = gz as usize;
    assert_eq!(body.voxel_map.get(x, y, z), 1.0);
}

#[test]
fn test_muscle_attenuation() {
    let frame = PoseFrame::default();
    let body = HumanBody::from_pose(&frame, 32, 0.1);
    let field = Complex::new(1.0, 0.0);
    let attenuated = body.attenuate_rf_field(field, 0.1, 2.4e9); // 10cm distance
    assert!(attenuated.norm() < 1.0); // Loss occurs
}

#[test]
fn test_shielding_facing() {
    let frame = PoseFrame::default();
    let body = HumanBody::from_pose(&frame, 32, 0.1);

    let shield_away = body.shielding_factor(std::f32::consts::PI, 0.0); // facing away
    let shield_toward = body.shielding_factor(0.0, 0.0); // facing toward

    assert!(shield_away > shield_toward); // Higher shielding when facing away
}
