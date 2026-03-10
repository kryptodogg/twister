# Check: Contrastive loss temperature τ = 0.07
if ! grep -q "temperature.*0.07\|0\.07" src/ml/timegnn_trainer.rs; then
    echo "❌ ERROR: Contrastive loss temperature must be 0.07 (generation-critical)"
fi

# Check: Silhouette threshold >= 0.6
if ! grep -q "silhouette.*<.*0.6\|< 0\.6" src/ml/pattern_discovery.rs; then
    echo "⚠️  WARNING: Silhouette threshold below 0.6 (cluster quality at risk)"
fi

# Check: Temporal frequency detection included
if ! grep -q "detect_temporal_periodicity\|temporal" src/ml/pattern_discovery.rs; then
    echo "❌ ERROR: Temporal frequency analysis missing (patterns invisible without it)"
fi

# Run tests
cargo test timegnn_training --lib -- --nocapture
if [ $? -ne 0 ]; then
    echo "❌ Tests failed"
fi

echo "✅ Track K validation passed"
