pub mod copilot_handler;
pub use copilot_handler::*;

pub mod transparency;
pub use transparency::{compositor_supports_transparency, enable_acrylic_blur, get_window_background_color};
