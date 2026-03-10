fn main() {
    // Only link RTL-SDR library if the feature is enabled
    if std::env::var("CARGO_FEATURE_RTLSDR").is_ok() {
        if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
            println!("cargo:rustc-link-search=native=.");
            println!("cargo:rustc-link-lib=static=rtlsdr");
        }
    }

    slint_build::compile("ui/app.slint").expect("Slint build failed");
    slint_build::compile("ui/applets/auto_waveshaping.slint").expect("Slint build failed");
    slint_build::compile("ui/applets/spectral_ingester.slint").expect("Slint build failed");
    slint_build::compile("ui/applets/ps5_deck.slint").expect("Slint build failed");
    slint_build::compile("ui/applets/joycon_wand.slint").expect("Slint build failed");
    slint_build::compile("ui/applets/toto_hud.slint").expect("Slint build failed");
    // slint_build::compile("ui/applets/chronos_training.slint").expect("Slint build failed"); // TODO: Fix Slint syntax errors
    slint_build::compile("ui/applets/mamba_brain.slint").expect("Slint build failed");
    slint_build::compile("assets/prototyping/toto/toto.slint").expect("Slint build failed");
}
