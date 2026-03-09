cat << 'INNER_EOF' > patch.py
import re
with open('src/main.rs', 'r') as f:
    content = f.read()

# Fix the duplicate fields in main.rs
content = re.sub(
r'''                impulse_detection: None,
                video_frame: None,
                video_frame_timestamp_us: 0,
                visual_features: None,
                anc_phase: None,
                harmonic_energy: None,
                impulse_detection: None,
                video_frame: None,
                video_frame_timestamp_us: 0,
                visual_features: None,''',
r'''                impulse_detection: None,
                video_frame: None,
                video_frame_timestamp_us: 0,
                visual_features: None,
                anc_phase: None,
                harmonic_energy: None,''',
content)

# Fix ForensicLogger initialization
old_init = '''    let forensic = Arc::new(std::sync::Mutex::new(
        ForensicLogger::new(session_identity.as_str()).context("Forensic log init")?,
    ));
    state.log(
        "INFO",
        "Forensic",
        &format!("Log: {}", forensic.lock().unwrap().log_path().display()),
    );'''

new_init = '''    let forensic = Arc::new(tokio::sync::Mutex::new(
        ForensicLogger::new(session_identity.as_str()).await.context("Forensic log init")?,
    ));

    // Log SessionStart
    if let Ok(f) = forensic.try_lock() {
        let start_ev = crate::forensic::ForensicEvent::SessionStart {
            timestamp_micros: crate::forensic::get_current_micros(),
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            total_events_prior: 0,
        };
        let _ = f.log(start_ev);
        state.log(
            "INFO",
            "Forensic",
            &format!("Log: {}", f.log_path().display()),
        );
    }'''
content = content.replace(old_init, new_init)

# Fix forensic lock logic in main.rs
old_log = '''                    if let Ok(mut f) = forensic_disp.lock() {
                        let _ = f.log_detection(&enriched_event);
                    }'''
new_log = '''                    let f_disp = forensic_disp.clone();
                    let ev_copy = enriched_event.clone();
                    tokio::spawn(async move {
                        if let Ok(f) = f_disp.try_lock() {
                            let _ = f.log_detection(&ev_copy);
                        }
                    });'''
content = content.replace(old_log, new_log)

old_log_2 = '''                                        if let Ok(mut f) = fdc.lock() {
                                            let _ = f.log_detection(&top.event);
                                        }'''
new_log_2 = '''                                        let fdc_copy = fdc.clone();
                                        let ev_copy = top.event.clone();
                                        tokio::spawn(async move {
                                            if let Ok(f) = fdc_copy.try_lock() {
                                                let _ = f.log_detection(&ev_copy);
                                            }
                                        });'''
content = content.replace(old_log_2, new_log_2)

old_log_3 = '''                                if let Ok(mut f) = fdc.lock() {
                                    println!(
                                        "[DEFENSE] EVT:{} DC:{:.2}v RF:{:.1}MHz",
                                        eid2,
                                        audio_bias,
                                        rf_hz / 1e6
                                    );
                                    if let Some(ev) = ecl {
                                        let _ = f.log_detection(&ev);
                                    }
                                }'''
new_log_3 = '''                                let fdc_copy = fdc.clone();
                                tokio::spawn(async move {
                                    if let Ok(f) = fdc_copy.try_lock() {
                                        println!(
                                            "[DEFENSE] EVT:{} DC:{:.2}v RF:{:.1}MHz",
                                            eid2,
                                            audio_bias,
                                            rf_hz / 1e6
                                        );
                                        if let Some(ev) = ecl {
                                            let _ = f.log_detection(&ev);
                                        }
                                    }
                                });'''
content = content.replace(old_log_3, new_log_3)


# Fix ui.on_export_evidence
old_export = '''                if let Ok(f) = f.lock() {
                    match f.export_evidence_report(
                        &path,
                        &case,
                        "Operator",
                        "Galveston TX",
                        None,
                        None,
                    ) {
                        Ok(_) => println!("[Forensic] Exported: {}", path),
                        Err(e) => eprintln!("[Forensic] Export failed: {e}"),
                    }
                }'''
new_export = '''                if let Ok(f) = f.try_lock() {
                    match f.export_evidence_report(
                        &path,
                        &case,
                        "Operator",
                        "Galveston TX",
                        None,
                        None,
                    ) {
                        Ok(_) => println!("[Forensic] Exported: {}", path),
                        Err(e) => eprintln!("[Forensic] Export failed: {e}"),
                    }
                }'''
content = content.replace(old_export, new_export)


# Final shutdown
old_final = '''    // Final evidence report
    {
        std::fs::create_dir_all("evidence").ok();
        let ts = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let path = format!("evidence/final_{ts}.html");
        if let Ok(f) = forensic.lock() {
            match f.export_evidence_report(
                &path,
                "TWISTER_FINAL",
                "Operator",
                "Galveston TX",
                None,
                None,
            ) {
                Ok(_) => println!("[Forensic] Final report: {}", path),
                Err(e) => eprintln!("[Forensic] Final export failed: {e}"),
            }
        }
    }

    println!("[Twister] Shutdown complete.");'''
new_final = '''    // Final evidence report
    {
        std::fs::create_dir_all("evidence").ok();
        let ts = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let path = format!("evidence/final_{ts}.html");

        let f = Arc::into_inner(forensic).unwrap().into_inner();

        match f.export_evidence_report(
            &path,
            "TWISTER_FINAL",
            "Operator",
            "Galveston TX",
            None,
            None,
        ) {
            Ok(_) => println!("[Forensic] Final report: {}", path),
            Err(e) => eprintln!("[Forensic] Final export failed: {e}"),
        }

        let _ = f.shutdown().await;
    }

    println!("[Twister] Shutdown complete.");'''
content = content.replace(old_final, new_final)

with open('src/main.rs', 'w') as f:
    f.write(content)
INNER_EOF
python3 patch.py
cargo check
