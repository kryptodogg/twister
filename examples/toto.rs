// examples/toto.rs
// Minimal runner for the Hardware-Locked Live Runner

use twister::ui::toto_logic::setup_toto_app;
use twister::state::AppState;
use std::sync::Arc;
use slint::ComponentHandle;

slint::include_modules!();

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let state = Arc::new(AppState::new());
    let ui = TotoCard::new()?;

    setup_toto_app(&ui, state);

    ui.run()?;
    Ok(())
}
