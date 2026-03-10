use std::fs;
use std::path::Path;

fn main() {
    // For Windows, link against the prebuilt rtlsdr.lib in the root directory
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        println!("cargo:rustc-link-search=native=.");
        println!("cargo:rustc-link-lib=static=rtlsdr");

        // Link against libusb-1.0 (required by RTL-SDR)
        // Add search path to third-party libusb directory
        let libusb_path = "third_party/libusb-1.0.30-rc1/VS2015/MS64/dll";
        println!("cargo:rustc-link-search=native={}", libusb_path);
        println!("cargo:rustc-link-lib=dylib=libusb-1.0");

        // Copy DLLs to target directory so they're available at runtime
        let out_dir = std::env::var("OUT_DIR").unwrap();
        let target_dir = Path::new(&out_dir)
            .ancestors()
            .find(|p| p.ends_with("debug") || p.ends_with("release"))
            .unwrap_or_else(|| Path::new(&out_dir));

        // Copy libusb-1.0.dll if it exists
        let libusb_dll = format!("{}/libusb-1.0.dll", libusb_path);
        let dest_dll = target_dir.join("libusb-1.0.dll");
        if Path::new(&libusb_dll).exists() && !dest_dll.exists() {
            let _ = fs::copy(&libusb_dll, &dest_dll);
        }
    }

    slint_build::compile("ui/app.slint").expect("Slint build failed");
}
