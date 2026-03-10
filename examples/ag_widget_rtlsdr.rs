slint::include_modules!();
use std::time::Duration;
use tokio::time;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ui = RtlSdrWidget::new()?;
    let backend = ui.global::<RtlSdrBackend>();

    backend.on_select_antenna(move |antenna| {
        println!("[RTL-SDR] Antenna physically switched to: {}", antenna);
    });

    let ui_handle = ui.as_weak();
    
    // Simulating the Track B Dispatch Loop polling
    tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_millis(100)); // Simulating a slower blink for visual effect
        let mut toggle = false;
        
        loop {
            interval.tick().await;
            toggle = !toggle;
            
            if let Some(ui) = ui_handle.upgrade() {
                let backend = ui.global::<RtlSdrBackend>();
                // This mimics the dirty_flags.rf_data_available getting set by the 10ms loop
                backend.set_rf_data_available(toggle); 
            }
        }
    });

    ui.run()?;
    Ok(())
}
