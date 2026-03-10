slint::include_modules!();

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ui = PlutoWidget::new()?;
    let backend = ui.global::<PlutoBackend>();

    backend.on_set_modulation(move |mod_type| {
        println!("[Pluto+] Modulation switched to: {}", mod_type);
        // This will eventually update the DMA gateway chunk logic
    });

    backend.on_trigger_tx(move || {
        println!("[Pluto+] Firing TX Burst at configured power and frequency");
    });

    ui.run()?;
    Ok(())
}
