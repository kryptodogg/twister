fn main() {
    if std::env::var("CARGO_FEATURE_RTLSDR").is_ok() {
        if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
            println!("cargo:rustc-link-search=native=.");
            println!("cargo:rustc-link-lib=static=rtlsdr");
        }
    }

    slint_build::compile("ui/toto.slint").expect("Slint build failed");
    slint_build::compile("ui/hardware.slint").expect("Slint build failed");
}
