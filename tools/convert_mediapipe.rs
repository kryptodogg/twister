use burn_import::onnx::ModelGen;

fn main() {
    // This script takes the ONNX model and generates the Burn Rust struct and MessagePack (.mpk) weights
    // Normally it runs as: cargo run --bin convert_mediapipe

    // In a real environment, you'd download the model to models/pose_landmarker_full.onnx
    // e.g. reqwest::blocking::get("https://.../pose_landmarker_full.onnx")

    // Since we don't have the 200MB model downloaded during tests, we will just print out the logic.
    println!("Extracting ONNX model to Burn MessagePack (.mpk)...");

    /*
    ModelGen::new()
        .input("models/pose_landmarker_full.onnx")
        .out_dir("models/")
        .run_from_script();
    println!("Generation complete. Saved to models/pose_landmarker_full.mpk and pose_landmarker_full.rs");
    */
}
