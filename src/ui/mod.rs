pub mod copilot_handler;
pub use copilot_handler::*;

pub mod fonts;
pub use fonts::register_default_fonts;

pub mod transparency;
pub use transparency::{compositor_supports_transparency, enable_acrylic_blur, get_window_background_color};

pub mod emeraldcity;
pub use emeraldcity::{frequency_to_rgb, get_resonant_color, resonant_fold_hz, RESONANT_LOWER_HZ, RESONANT_UPPER_HZ};
pub mod colorofspectrum;

pub mod toto_widget;
pub use toto_widget::TotoWidget;

pub mod chronos_widget;
pub use chronos_widget::ChronosWidget;

pub mod app_controller;

