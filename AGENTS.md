# Twister Project Agents and Toolchain Configuration

## Rust Toolchain Configuration

### Current Setup
- **Rust Edition**: 2024
- **Toolchain**: Nightly
- **Policy**: No toolchain pinning or overrides enforced
- **Configuration**: Managed via `rust-toolchain.toml`

### Toolchain Components
The project includes the following essential development tools:
- `rustfmt` - Code formatting
- `clippy` - Linting and code analysis
- `rust-src` - Source code for standard library (required for some tools)

### Configuration File
```toml
[toolchain]
channel = "nightly"
components = ["rustfmt", "clippy", "rust-src"]
```

### Policy Details

#### No Toolchain Pinning
- **Flexibility**: Developers can use their preferred Rust version
- **No Overrides**: No forced toolchain constraints at the project level
- **Developer Choice**: Team members can work with their preferred setup

#### Nightly Features
- **Cutting-edge**: Access to latest Rust language features
- **Experimental**: Support for unstable features when needed
- **Innovation**: Enables use of advanced Rust capabilities

#### Compatibility
- **Stable Support**: Backward compatibility where possible
- **Gradual Adoption**: New features adopted incrementally
- **Risk Management**: Careful evaluation of nightly-only dependencies

### Benefits
1. **Maximum Flexibility**: No forced constraints on developer environments
2. **Feature Access**: Full access to Rust's latest capabilities
3. **Development Speed**: Reduced friction in the development workflow
4. **Future-Proof**: Ready for Rust 2024 edition features

### Usage Guidelines
- Use nightly toolchain for development and CI
- Test critical paths with stable Rust when possible
- Document any nightly-only features in code comments
- Consider stability when adding new dependencies

## Agent Development Standards

### Code Quality
- All code must pass `cargo fmt` formatting
- Clippy warnings should be addressed
- Nightly features require justification in PR descriptions

### Testing
- Unit tests required for all new functionality
- Integration tests for critical paths
- Performance benchmarks for performance-sensitive code

### Documentation
- Nightly-only features must be documented
- Toolchain requirements clearly stated in relevant files
- Migration paths documented for breaking changes