fn main() {
    // For Windows, link against the prebuilt rtlsdr.lib in the root directory
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        println!("cargo:rustc-link-search=native=.");
        println!("cargo:rustc-link-lib=static=rtlsdr");
        // Link against libusb-1.0 (required by RTL-SDR)
        println!("cargo:rustc-link-lib=dylib=libusb-1.0");
    }

    slint_build::compile("ui/app.slint").expect("Slint build failed");
}
