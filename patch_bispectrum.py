with open('src/forensic.rs', 'r') as f:
    content = f.read()

# Replace LegacyBispectrum with Bispectrum
old_enum = '''    LegacyBispectrum {
        timestamp_micros: u64,
        f1_hz: f32,
        f2_hz: f32,
        product_hz: f32,
        magnitude: f32,
        coherence_frames: u32,
        confidence: f32,
    }'''

new_enum = '''    Bispectrum {
        timestamp_micros: u64,
        f1_hz: f32,
        f2_hz: f32,
        product_hz: f32,
        magnitude: f32,
        coherence_frames: u32,
        confidence: f32,
    }'''
content = content.replace(old_enum, new_enum)

old_val = '''            ForensicEvent::LegacyBispectrum {'''
new_val = '''            ForensicEvent::Bispectrum {'''
content = content.replace(old_val, new_val)

old_match_ts = '''ForensicEvent::LegacyBispectrum { timestamp_micros, .. } => *timestamp_micros,'''
new_match_ts = '''ForensicEvent::Bispectrum { timestamp_micros, .. } => *timestamp_micros,'''
content = content.replace(old_match_ts, new_match_ts)

old_log_det = '''        let fe = ForensicEvent::LegacyBispectrum {
            timestamp_micros: get_current_micros(),
            f1_hz: event.f1_hz,
            f2_hz: event.f2_hz,
            product_hz: event.product_hz,
            magnitude: event.magnitude,
            coherence_frames: event.coherence_frames,
            confidence,
        };'''
new_log_det = '''        let fe = ForensicEvent::Bispectrum {
            timestamp_micros: get_current_micros(),
            f1_hz: event.f1_hz,
            f2_hz: event.f2_hz,
            product_hz: event.product_hz,
            magnitude: event.magnitude,
            coherence_frames: event.coherence_frames,
            confidence,
        };'''
content = content.replace(old_log_det, new_log_det)

with open('src/forensic.rs', 'w') as f:
    f.write(content)
