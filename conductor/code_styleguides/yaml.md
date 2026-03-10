# YAML Style Guide

## Purpose
Define YAML configuration standards for Project Twister, ensuring consistency across CI/CD, Docker, and application configuration files.

## Core Principles
1. **Explicit Types**: Always quote strings that could be misinterpreted
2. **Consistent Indentation**: 2 spaces, no tabs
3. **Alphabetical Ordering**: Keys in alphabetical order (unless logical grouping required)
4. **Environment Separation**: Use anchors/aliases for DRY configs

## Basic Syntax

### Indentation
```yaml
# ✅ GOOD: 2-space indentation
dsp:
  fft_size: 2048
  sample_rate: 192000
  oversample: 64

# ❌ BAD: 4-space indentation
dsp:
    fft_size: 2048
```

### Quoting Strings
```yaml
# ✅ GOOD: Quote strings that could be ambiguous
version: "0.5.0"        # Not float 0.5
sample_rate: "192000"   # Not octal 192000
center_freq: "433.92MHz"

# ❌ BAD: Unquoted strings
version: 0.5.0          # Parsed as float
sample_rate: 192000     # Parsed as integer
```

### Comments
```yaml
# ✅ GOOD: Inline comments for complex values
dsp:
  fft_size: 2048  # Must be power of 2
  window: hann    # Hann window for spectral leakage reduction

# Top-level section comments
# ── GPU Configuration ────────────────────────────────────────────
gpu:
  device: "discrete_gpu_0"
```

## Docker Compose Patterns

### Service Definition
```yaml
# docker-compose.yml
version: "3.8"

services:
  neo4j:
    image: neo4j:5.15-enterprise
    container_name: twister-neo4j
    environment:
      NEO4J_AUTH: "neo4j/${NEO4J_PASSWORD}"
      NEO4J_PLUGINS: '["apoc"]'
    ports:
      - "7474:7474"  # HTTP
      - "7687:7687"  # Bolt
    volumes:
      - neo4j_data:/data
    networks:
      - twister-net
    healthcheck:
      test: ["CMD", "neo4j-admin", "dbms", "report"]
      interval: 30s
      timeout: 10s
      retries: 3

  qdrant:
    image: qdrant/qdrant:v1.7.0
    container_name: twister-qdrant
    ports:
      - "6333:6333"  # REST
      - "6334:6334"  # gRPC
    volumes:
      - qdrant_data:/qdrant/storage
    networks:
      - twister-net

networks:
  twister-net:
    driver: bridge

volumes:
  neo4j_data:
  qdrant_data:
```

### Build Configuration
```yaml
# docker-compose.yml (build section)
services:
  twister:
    build:
      context: .
      dockerfile: Dockerfile
      args:
        RUST_VERSION: "1.75"
        CUDA_VERSION: "12.0"  # For GPU acceleration
      target: production      # Multi-stage build
    profiles:
      - full                  # Optional service
    depends_on:
      neo4j:
        condition: service_healthy
      qdrant:
        condition: service_started
```

## CI/CD Configuration

### GitHub Actions
```yaml
# .github/workflows/ci.yml
name: CI

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main]

env:
  CARGO_TERM_COLOR: always
  RUST_VERSION: "1.75"

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - name: Checkout
        uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-action@stable
        with:
          toolchain: ${{ env.RUST_VERSION }}
          components: clippy, rustfmt

      - name: Cache Cargo
        uses: Swatinem/rust-cache@v2
        with:
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}

      - name: Build
        run: cargo build --release

      - name: Test
        run: cargo test --release

      - name: Clippy
        run: cargo clippy -- -D warnings

      - name: Format Check
        run: cargo fmt -- --check
```

## Application Configuration

### DSP Configuration
```yaml
# config/dsp.yml
dsp:
  fft:
    size: 2048
    window: "hann"
    overlap: 0.5

  pdm:
    oversample_ratio: 64
    decimation_order: 5
    sample_rate: 192000

  bispectrum:
    enabled: true
    threshold: 0.3
    min_coherence_frames: 10
```

### GPU Configuration
```yaml
# config/gpu.yml
gpu:
  backend: "wgpu"
  device:
    type: "discrete_gpu"
    index: 0  # RX 6700 XT

  memory:
    reserved_vram_mb: 2048
    unified_memory: true  # AMD SAM

  waterfall:
    cols: 128
    rows: 64
    scale: 2  # Bilinear upscale to 256×128

  bispectrum:
    sparse: true
    threshold: 0.1
```

### ML Configuration
```yaml
# config/ml.yml
ml:
  encoder:
    type: "mamba"
    d_model: 128
    n_layers: 4
    d_state: 16
    d_latent: 32

  classifier:
    type: "candle"
    precision: "f16"
    quantization: false

  training:
    batch_size: 16
    learning_rate: 0.001
    max_epochs: 100
    early_stopping: true
    patience: 10
```

## Anchors and Aliases

### DRY Configuration
```yaml
# ✅ GOOD: Reuse common settings
defaults: &defaults
  restart: unless-stopped
  logging:
    driver: json-file
    options:
      max-size: "10m"
      max-file: "3"

services:
  neo4j:
    <<: *defaults
    image: neo4j:5.15-enterprise

  qdrant:
    <<: *defaults
    image: qdrant/qdrant:v1.7.0
```

### Environment-Specific Overrides
```yaml
# config/production.yml
production: &production
  gpu:
    reserved_vram_mb: 4096  # More VRAM for production
  logging:
    level: "warn"           # Less verbose

development: &development
  gpu:
    reserved_vram_mb: 2048
  logging:
    level: "debug"

# Use with merge key
services:
  twister:
    <<: *production
```

## Validation

### Schema Validation
```yaml
# .github/workflows/validate.yml
validate:
  runs-on: ubuntu-latest
  steps:
    - name: Validate Docker Compose
      run: docker-compose config

    - name: Validate YAML Syntax
      uses: ibiqlik/action-yamllint@v3
      with:
        config_data: |
          rules:
            line-length: disable
            truthy: disable
```

## Common Pitfalls

### Boolean Traps
```yaml
# ✅ GOOD: Explicit booleans
enabled: true
disabled: false

# ❌ BAD: Ambiguous values
enabled: yes      # Parsed as string "yes"
disabled: no      # Parsed as string "no"
```

### Null Values
```yaml
# ✅ GOOD: Explicit null
optional_field: null

# ❌ BAD: Implicit null
optional_field: ~   # Works but less clear
optional_field:     # Also null, but confusing
```

### List Syntax
```yaml
# ✅ GOOD: Consistent list style
ports:
  - "7474:7474"
  - "7687:7687"

# Or inline for short lists
ports: ["7474:7474", "7687:7687"]

# ❌ BAD: Mixed styles
ports:
  - "7474:7474"
  - "7687:7687"
  - ["6333:6333", "6334:6334"]  # Don't mix
```

## References
- [YAML Specification](https://yaml.org/spec/1.2.2/)
- [Docker Compose Reference](https://docs.docker.com/compose/compose-file/)
- [GitHub Actions Workflow Syntax](https://docs.github.com/en/actions/writing-workflows/workflow-syntax-for-github-actions)
