use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BodyRegion {
    Head,
    Mouth,
    Torso,
    LeftArm,
    RightArm,
    LeftLeg,
    RightLeg,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MaterialProps {
    pub hardness: f32,    // [0, 1]
    pub roughness: f32,   // [0, 1]
    pub wetness: f32,     // [0, 1]
}

pub struct BodyRegionClassifier {
    region_map: HashMap<usize, BodyRegion>,
}

impl BodyRegionClassifier {
    pub fn new() -> Self {
        let mut region_map = HashMap::new();
        // Nose, Left/Right Eye
        for i in 0..=8 { region_map.insert(i, BodyRegion::Head); }
        // Mouth
        region_map.insert(9, BodyRegion::Mouth);
        region_map.insert(10, BodyRegion::Mouth);
        // Shoulders/Torso
        region_map.insert(11, BodyRegion::Torso);
        region_map.insert(12, BodyRegion::Torso);
        region_map.insert(23, BodyRegion::Torso);
        region_map.insert(24, BodyRegion::Torso);
        // Arms
        region_map.insert(13, BodyRegion::LeftArm);
        region_map.insert(15, BodyRegion::LeftArm);
        region_map.insert(17, BodyRegion::LeftArm);
        region_map.insert(19, BodyRegion::LeftArm);
        region_map.insert(21, BodyRegion::LeftArm);

        region_map.insert(14, BodyRegion::RightArm);
        region_map.insert(16, BodyRegion::RightArm);
        region_map.insert(18, BodyRegion::RightArm);
        region_map.insert(20, BodyRegion::RightArm);
        region_map.insert(22, BodyRegion::RightArm);
        // Legs
        region_map.insert(25, BodyRegion::LeftLeg);
        region_map.insert(27, BodyRegion::LeftLeg);
        region_map.insert(29, BodyRegion::LeftLeg);
        region_map.insert(31, BodyRegion::LeftLeg);

        region_map.insert(26, BodyRegion::RightLeg);
        region_map.insert(28, BodyRegion::RightLeg);
        region_map.insert(30, BodyRegion::RightLeg);
        region_map.insert(32, BodyRegion::RightLeg);

        Self { region_map }
    }

    pub fn get_region(&self, keypoint_idx: usize) -> BodyRegion {
        self.region_map.get(&keypoint_idx).copied().unwrap_or(BodyRegion::Torso)
    }

    pub fn region_to_material(&self, region: BodyRegion) -> MaterialProps {
        match region {
            BodyRegion::Head => MaterialProps {
                hardness: 0.8,
                roughness: 0.3,
                wetness: 0.6,
            },
            BodyRegion::Mouth => MaterialProps {
                hardness: 0.3,
                roughness: 0.4,
                wetness: 0.8,
            },
            BodyRegion::Torso => MaterialProps {
                hardness: 0.5,
                roughness: 0.6,
                wetness: 0.4,
            },
            BodyRegion::LeftArm | BodyRegion::RightArm => MaterialProps {
                hardness: 0.4,
                roughness: 0.5,
                wetness: 0.3,
            },
            BodyRegion::LeftLeg | BodyRegion::RightLeg => MaterialProps {
                hardness: 0.3,
                roughness: 0.7,
                wetness: 0.2,
            },
        }
    }

    pub fn motion_modulate_material(
        &self,
        base_material: MaterialProps,
        velocity: f32,
    ) -> MaterialProps {
        // Fast motion -> hardness decreases (joint "exposed")
        let hardness_factor = (1.0 - velocity.min(1.0) * 0.3).max(0.3);

        MaterialProps {
            hardness: base_material.hardness * hardness_factor,
            roughness: base_material.roughness,
            wetness: base_material.wetness,
        }
    }
}
