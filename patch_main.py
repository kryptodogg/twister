import re

with open('src/main.rs', 'r') as f:
    content = f.read()

# Replace initialization
old_init = '''    let forensic = Arc::new(tokio::sync::Mutex::new(
        ForensicLogger::new(session_identity.as_str()).await.map_err(|e| anyhow::anyhow!("{:?}", e)).context("Forensic log init")?,
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

new_init = '''    let forensic = ForensicLogger::new(session_identity.as_str()).await.map_err(|e| anyhow::anyhow!("{:?}", e)).context("Forensic log init")?;

    // Log SessionStart
    let start_ev = crate::forensic::ForensicEvent::SessionStart {
        timestamp_micros: crate::forensic::get_current_micros(),
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        total_events_prior: 0,
    };
    let _ = forensic.log(start_ev);
    state.log(
        "INFO",
        "Forensic",
        &format!("Log: {}", forensic.log_path().display()),
    );'''
content = content.replace(old_init, new_init)

# Fix logs inside dispatch loop
old_log_1 = '''                    let f_disp = forensic_disp.clone();
                    let ev_copy = enriched_event.clone();
                    tokio::spawn(async move {
                        if let Ok(f) = f_disp.try_lock() {
                            let _ = f.log_detection(&ev_copy);
                        }
                    });'''
new_log_1 = '''                    let _ = forensic_disp.log_detection(&enriched_event);'''
content = content.replace(old_log_1, new_log_1)


old_log_2 = '''                                        let fdc_copy = fdc.clone();
                                        let ev_copy = top.event.clone();
                                        tokio::spawn(async move {
                                            if let Ok(f) = fdc_copy.try_lock() {
                                                let _ = f.log_detection(&ev_copy);
                                            }
                                        });'''
new_log_2 = '''                                        let _ = fdc.log_detection(&top.event);'''
content = content.replace(old_log_2, new_log_2)


old_log_3 = '''                                let fdc_copy = fdc.clone();
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
new_log_3 = '''                                println!(
                                    "[DEFENSE] EVT:{} DC:{:.2}v RF:{:.1}MHz",
                                    eid2,
                                    audio_bias,
                                    rf_hz / 1e6
                                );
                                if let Some(ev) = ecl {
                                    let _ = fdc.log_detection(&ev);
                                }'''
content = content.replace(old_log_3, new_log_3)

# Fix export button hook
old_export = '''                if let Ok(f) = f.try_lock() {
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
new_export = '''                match f.export_evidence_report(
                    &path,
                    &case,
                    "Operator",
                    "Galveston TX",
                    None,
                    None,
                ) {
                    Ok(_) => println!("[Forensic] Exported: {}", path),
                    Err(e) => eprintln!("[Forensic] Export failed: {e}"),
                }'''
content = content.replace(old_export, new_export)

# Fix shutdown
old_shutdown = '''    // Final evidence report
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
new_shutdown = '''    // Final evidence report
    {
        std::fs::create_dir_all("evidence").ok();
        let ts = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let path = format!("evidence/final_{ts}.html");

        match forensic.export_evidence_report(
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

        let _ = forensic.shutdown().await;
    }

    println!("[Twister] Shutdown complete.");'''
content = content.replace(old_shutdown, new_shutdown)

with open('src/main.rs', 'w') as f:
    f.write(content)
