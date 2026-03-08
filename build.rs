fn main() {
    // For Windows, link against the prebuilt rtlsdr.lib in the root directory
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        println!("cargo:rustc-link-search=native=.");
        println!("cargo:rustc-link-lib=static=rtlsdr");
    }

    slint_build::compile("ui/app.slint").expect("Slint build failed");
}
