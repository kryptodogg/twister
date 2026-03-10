sed -i '98,$d' examples/field_physics_bench.rs
cat << 'INNER' >> examples/field_physics_bench.rs
    // Check JSON serialization functionality too!
    let output_path = "test_pattern_library.json";
    library.save(output_path).unwrap();
    let loaded_library = pattern_discovery::load_pattern_library(output_path).unwrap();
    assert_eq!(library.total_patterns, loaded_library.total_patterns);
    println!("✅ JSON serialization successful: saved to {}", output_path);
}
INNER
