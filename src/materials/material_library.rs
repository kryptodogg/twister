use serde::{Serialize, Deserialize};
use std::collections::HashMap;

#[repr(C)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MaterialDef {
    pub material_id: u32,
    pub name: [u8; 32],

    pub permittivity_static: f32,
    pub permittivity_infinity: f32,
    pub relaxation_time_ps: f32,

    pub conductivity_base: f32,
    pub conductivity_frequency_exp: f32,

    pub loss_tangent_1ghz: f32,
    pub absorption_coefficient: f32,

    pub density_kg_m3: f32,
    pub acoustic_impedance: f32,

    pub roughness: f32,
    pub anisotropy: f32,

    pub thermal_conductivity: f32,
    pub specific_heat: f32,

    pub confidence: f32,
    pub last_updated_micros: u64,
    pub version: u32,

    pub reserved: [u8; 32],
}

#[repr(C)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MaterialPoint {
    pub position_xyz: [f32; 3],

    pub material_id: u32,
    pub material_blend: f32,
    pub next_material_id: u32,

    pub confidence: f32,
    pub timestamp_micros: u64,

    pub permittivity_at_freq: f32,
    pub conductivity_at_freq: f32,

    pub velocity_xyz: [f32; 3],
    pub temperature_kelvin: f32,

    pub attenuation_db_per_cm: f32,
    pub group_velocity_ratio: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MaterialLibrary {
    pub materials: HashMap<String, MaterialDef>,
}

impl MaterialLibrary {
    pub fn default() -> Self {
        let mut lib = MaterialLibrary {
            materials: HashMap::new(),
        };

        // Helper to copy strings into fixed arrays
        let to_name = |s: &str| -> [u8; 32] {
            let mut arr = [0u8; 32];
            let bytes = s.as_bytes();
            let len = std::cmp::min(bytes.len(), 31);
            arr[..len].copy_from_slice(&bytes[..len]);
            arr
        };

        // Water
        lib.materials.insert("water".to_string(), MaterialDef {
            material_id: 0,
            name: to_name("water"),
            permittivity_static: 80.0,
            permittivity_infinity: 4.8,
            relaxation_time_ps: 8.3,
            conductivity_base: 0.05,
            conductivity_frequency_exp: 0.5,
            loss_tangent_1ghz: 0.15,
            absorption_coefficient: 0.02,
            density_kg_m3: 1000.0,
            acoustic_impedance: 1.48e6,
            roughness: 0.0,
            anisotropy: 0.0,
            thermal_conductivity: 0.6,
            specific_heat: 4200.0,
            confidence: 0.99,
            last_updated_micros: 0,
            version: 1,
            reserved: [0; 32],
        });

        // Glass
        lib.materials.insert("glass".to_string(), MaterialDef {
            material_id: 1,
            name: to_name("glass"),
            permittivity_static: 6.0,
            permittivity_infinity: 5.8,
            relaxation_time_ps: 1e-3,
            conductivity_base: 1e-11,
            conductivity_frequency_exp: 1.0,
            loss_tangent_1ghz: 0.001,
            absorption_coefficient: 0.0001,
            density_kg_m3: 2230.0,
            acoustic_impedance: 1.26e7,
            roughness: 0.05,
            anisotropy: 0.0,
            thermal_conductivity: 1.2,
            specific_heat: 840.0,
            confidence: 0.98,
            last_updated_micros: 0,
            version: 1,
            reserved: [0; 32],
        });

        // Human Tissue
        lib.materials.insert("human_tissue".to_string(), MaterialDef {
            material_id: 2,
            name: to_name("human_tissue"),
            permittivity_static: 50.0,
            permittivity_infinity: 4.0,
            relaxation_time_ps: 10.0,
            conductivity_base: 0.5,
            conductivity_frequency_exp: 0.4,
            loss_tangent_1ghz: 0.2,
            absorption_coefficient: 0.03,
            density_kg_m3: 1050.0,
            acoustic_impedance: 1.54e6,
            roughness: 0.3,
            anisotropy: 0.1,
            thermal_conductivity: 0.5,
            specific_heat: 3500.0,
            confidence: 0.85,
            last_updated_micros: 0,
            version: 1,
            reserved: [0; 32],
        });

        // Wood
        lib.materials.insert("wood".to_string(), MaterialDef {
            material_id: 3,
            name: to_name("wood"),
            permittivity_static: 3.5,
            permittivity_infinity: 3.4,
            relaxation_time_ps: 1e-2,
            conductivity_base: 1e-3,
            conductivity_frequency_exp: 0.8,
            loss_tangent_1ghz: 0.01,
            absorption_coefficient: 0.005,
            density_kg_m3: 500.0,
            acoustic_impedance: 2.4e6,
            roughness: 0.4,
            anisotropy: 0.2,
            thermal_conductivity: 0.12,
            specific_heat: 1500.0,
            confidence: 0.90,
            last_updated_micros: 0,
            version: 1,
            reserved: [0; 32],
        });

        // Metal
        lib.materials.insert("metal".to_string(), MaterialDef {
            material_id: 4,
            name: to_name("metal"),
            permittivity_static: -1e6,
            permittivity_infinity: 1.0,
            relaxation_time_ps: 0.1,
            conductivity_base: 3.8e7,
            conductivity_frequency_exp: 0.0,
            loss_tangent_1ghz: 1e-4,
            absorption_coefficient: 0.0,
            density_kg_m3: 2700.0,
            acoustic_impedance: 1.73e7,
            roughness: 0.2,
            anisotropy: 0.0,
            thermal_conductivity: 237.0,
            specific_heat: 900.0,
            confidence: 0.95,
            last_updated_micros: 0,
            version: 1,
            reserved: [0; 32],
        });

        lib
    }

    pub fn save_json(&self) -> Result<(), Box<dyn std::error::Error>> {
        std::fs::create_dir_all("@databases/materials").ok();
        let path = "@databases/materials/materials.json";
        let materials_json = serde_json::to_string_pretty(&self)?;
        std::fs::write(path, materials_json)?;
        Ok(())
    }

    pub fn load_json() -> Result<Self, Box<dyn std::error::Error>> {
        let path = "@databases/materials/materials.json";
        if std::path::Path::new(path).exists() {
            let json = std::fs::read_to_string(path)?;
            Ok(serde_json::from_str(&json)?)
        } else {
            Ok(Self::default())
        }
    }

    pub fn create_material(&mut self, name: &str, def: MaterialDef) {
        self.materials.insert(name.to_string(), def);
    }
}
