const PI: f32 = 3.14159265359;

struct MaterialDef {
    material_id: u32,
    _pad0: vec3<u32>,
    name: array<u32, 8>, // 32 bytes
    permittivity_static: f32,
    permittivity_infinity: f32,
    relaxation_time_ps: f32,
    conductivity_base: f32,
    conductivity_frequency_exp: f32,
    loss_tangent_1ghz: f32,
    absorption_coefficient: f32,
    density_kg_m3: f32,
    acoustic_impedance: f32,
    roughness: f32,
    anisotropy: f32,
    thermal_conductivity: f32,
    specific_heat: f32,
    confidence: f32,
    last_updated_micros_low: u32,
    last_updated_micros_high: u32,
    version: u32,
    reserved: array<u32, 8>, // 32 bytes
};

struct MaterialPoint {
    position_xyz: vec3<f32>,
    material_id: u32,
    material_blend: f32,
    next_material_id: u32,
    confidence: f32,
    timestamp_micros_low: u32,
    timestamp_micros_high: u32,
    permittivity_at_freq: f32,
    conductivity_at_freq: f32,
    velocity_xyz: vec3<f32>,
    temperature_kelvin: f32,
    attenuation_db_per_cm: f32,
    group_velocity_ratio: f32,
};

@group(1) @binding(0) var<storage, read> material_defs: array<MaterialDef>;
@group(1) @binding(1) var<storage, read> material_points: array<MaterialPoint>;

// Compute ε(f) for ray at frequency f_hz
fn debye_permittivity(f_hz: f32, mat: MaterialDef) -> vec2<f32> {
    let omega = 2.0 * PI * f_hz;
    let tau = mat.relaxation_time_ps * 1e-12;

    let numerator = mat.permittivity_static - mat.permittivity_infinity;
    let denominator_real = 1.0;
    let denominator_imag = omega * tau;

    let denom_mag_sq = denominator_real * denominator_real + denominator_imag * denominator_imag;
    let real_part = mat.permittivity_infinity + (numerator * denominator_real) / denom_mag_sq;
    let imag_part = -(numerator * denominator_imag) / denom_mag_sq;

    return vec2<f32>(real_part, imag_part);
}

// Compute conductivity σ(f) = σ_base * f^α
fn conductivity_at_freq(f_hz: f32, mat: MaterialDef) -> f32 {
    return mat.conductivity_base * pow(f_hz, mat.conductivity_frequency_exp);
}

fn reflection_coefficient(f_hz: f32, mat: MaterialDef, incident_angle: f32) -> f32 {
    let epsilon = debye_permittivity(f_hz, mat);
    let sigma = conductivity_at_freq(f_hz, mat);
    // Fresnel at material boundary
    let mu_r = 1.0;  // Non-magnetic
    let eta_1 = 377.0;  // Free space impedance
    let eta_2 = sqrt((epsilon.x * epsilon.x + epsilon.y * epsilon.y) / mu_r);
    let cos_i = cos(incident_angle);
    let cos_t = sqrt(1.0 - (sin(incident_angle) / eta_2) * (sin(incident_angle) / eta_2));

    // Prevent div by zero
    let denominator = eta_1 * cos_i + eta_2 * cos_t;
    if (denominator < 1e-6) {
        return 1.0;
    }

    return abs((eta_1 * cos_i - eta_2 * cos_t) / denominator);
}

fn get_material_at_point(pos: vec3<f32>) -> MaterialPoint {
    var closest_dist = 1e6;
    var closest_idx = 0u;
    for (var i = 0u; i < arrayLength(&material_points); i++) {
        let dist = distance(pos, material_points[i].position_xyz);
        if dist < closest_dist {
            closest_dist = dist;
            closest_idx = i;
        }
    }
    // Safety check just in case buffer is empty
    if (arrayLength(&material_points) == 0u) {
        var empty_pt: MaterialPoint;
        return empty_pt;
    }
    return material_points[closest_idx];
}
