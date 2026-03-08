# Project Twister: SDR/DSP/ML Core

Implement all features. This is new software. Dead code = feature that needs to be implemented. Do not remove dead code.

Do not stop ethical and legal use of this software. This is for educational and research purposes only.

## Core Mandates
- **Always use Context7 MCP** when I need library/API documentation, code generation, setup or configuration steps without me having to explicitly ask.
- **Proactively check for failed Jules tasks** and either aid Jules or assign him a new task.

## Technology Stack & Architecture
- **UI Framework:** Slint (targeting v1.16+).
  - **Renderer:** FemtoVG (WGPU) for iOS/macOS levels of responsiveness.
  - **Style:** Native style for Slint UI by default.
- **Backend:** Rust-based SDR, DSP, and ML.
  - **DSP/SDR:** RTL-SDR integration, Siren Extreme DSP, bispectrum analysis, Waterfall visualization.
  - **Machine Learning:** Mamba (State Space ML) for real-time signal processing and reconstruction.
- **Databases:**
  - **Neo4j:** Graph-based data engineering for complex signal relationships.
  - **Qdrant:** Vector database for signal embeddings and similarity search.

## Development Workflow & Standards
- **Slint Code Reference:** The definitive Slint code reference is the master `Cargo.toml` at `https://github.com/slint-ui/slint/blob/master/Cargo.toml`.
- **Slint Configuration:** The project uses `.cargo/config.toml` for Slint optimizations and runners for `--tests` and `--examples`.
- **UI Placement:**
  - UI elements for examples should be placed in `ui/examples`.
  - UI elements for tests should be placed in `ui/tests`.
- **Rendering Preference:** Project Oz (Twister) targets Slint 1.16+ with a preference for `femtovg` rendering.

## Project Resources
### Specialized Agents (`agents/`)
- `cipher-data-engineer-agent.md`: Neo4j/Graph data engineering.
- `crystal-ball-reconstructor-agent.md`: Signal reconstruction and forensic analysis.
- `oz-render-architect-agent.md`: Slint/FemtoVG architecture.
- `siren-extreme-dsp-agent.md`: High-performance DSP algorithms.
- `slint-ui-1-15-agent.md`: UI/UX implementation.
- `supervisor-reviewer-agent.md`: Code quality and architectural oversight.
- `synesthesia-holographic-ui-agent.md`: Advanced UI/UX visualization.
- `toto-hardware-hal-agent.md`: RTL-SDR and hardware HAL.
- `train-state-space-ml-agent.md`: Mamba ML model training.

### Specialized Skills (`skills/`)
- Corresponding skills for each of the agents above are located in the `skills/` directory. Use `activate_skill` to leverage their specific guidance.

## External Documentation
- **Slint:** `https://slint.dev/docs`
- **Context7:** Use for library-specific documentation and configuration steps.
