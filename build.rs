fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        println!("cargo:rustc-link-search=native=.");
        println!("cargo:rustc-link-lib=static=rtlsdr");
    }

    slint_build::compile("ui/app.slint").expect("Slint build failed");
    slint_build::compile("ui/applets/auto_waveshaping.slint").expect("Slint build failed");
    slint_build::compile("ui/applets/ps5_deck.slint").expect("Slint build failed");
    slint_build::compile("ui/applets/joycon_wand.slint").expect("Slint build failed");
}
