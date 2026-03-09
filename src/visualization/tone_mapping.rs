// src/visualization/tone_mapping.rs

pub fn tone_map_reinhard(linear: [f32; 3], exposure: f32, white_point: f32) -> [f32; 3] {
    let exposed = [
        linear[0] * exposure,
        linear[1] * exposure,
        linear[2] * exposure,
    ];

    [
        exposed[0] * (1.0 + exposed[0] / (white_point * white_point)) / (1.0 + exposed[0]),
        exposed[1] * (1.0 + exposed[1] / (white_point * white_point)) / (1.0 + exposed[1]),
        exposed[2] * (1.0 + exposed[2] / (white_point * white_point)) / (1.0 + exposed[2]),
    ]
}

pub fn tone_map_aces(linear: [f32; 3]) -> [f32; 3] {
    const A: f32 = 2.51;
    const B: f32 = 0.03;
    const C: f32 = 2.43;
    const D: f32 = 0.59;
    const E: f32 = 0.14;

    [
        apply_aces_curve(linear[0], A, B, C, D, E),
        apply_aces_curve(linear[1], A, B, C, D, E),
        apply_aces_curve(linear[2], A, B, C, D, E),
    ]
}

fn apply_aces_curve(x: f32, a: f32, b: f32, c: f32, d: f32, e: f32) -> f32 {
    (x * (a * x + b)) / (x * (c * x + d) + e)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reinhard_clipping() {
        let linear = [10.0, 50.0, 100.0];
        let mapped = tone_map_reinhard(linear, 1.0, 1.0);
        assert!(mapped[0] <= 1.0 && mapped[0] >= 0.0);
        assert!(mapped[1] <= 1.0 && mapped[1] >= 0.0);
        assert!(mapped[2] <= 1.0 && mapped[2] >= 0.0);
    }

    #[test]
    fn test_aces_color_accuracy() {
        let linear = [0.1, 0.5, 0.8];
        let mapped = tone_map_aces(linear);
        assert!(mapped[0] <= 1.0 && mapped[0] >= 0.0);
        assert!(mapped[1] <= 1.0 && mapped[1] >= 0.0);
        assert!(mapped[2] <= 1.0 && mapped[2] >= 0.0);
        assert!(mapped[2] > mapped[1]);
        assert!(mapped[1] > mapped[0]);
    }

    #[test]
    fn test_exposure_compensation() {
        let linear = [0.5, 0.5, 0.5];
        let low_exp = tone_map_reinhard(linear, 0.1, 100.0);
        let high_exp = tone_map_reinhard(linear, 10.0, 100.0);
        assert!(low_exp[0] < high_exp[0]);
    }
}
