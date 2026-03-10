---
id: path-implementation-pattern
trigger: "when implementing Project Oz paths"
confidence: 0.9
domain: "project-oz-workflow"
source: "session-observation-2026-02-25"
---

# Path Implementation Pattern for Project Oz

## Action
When implementing Project Oz paths, follow this pattern:

1. **Read existing code first** - Check if implementation already exists (like Path 6 SpectrumContainer)
2. **Add verification tests** - Don't duplicate working code, add comprehensive tests
3. **Create completion report** - Document success criteria, integration points, performance targets
4. **Commit with detailed message** - Include success criteria checklist, files modified, test coverage

## Evidence
- Path 6: Implementation existed, added 4 verification tests + completion report
- Path 10: Created SVGF denoiser (340 lines Rust + 280 lines WGSL) + tests + report
- Path 11: Created RF SDF refinement (340 lines WGSL) + completion report
- All paths committed with detailed messages including success criteria

## Related Instincts
- prefer-completion-reports: Always document path completion with success criteria verification
- add-tests-for-new-code: Every implementation needs unit tests before commit
- commit-atomic-paths: Each path gets its own commit with full documentation

---
id: rdna2-optimization-pattern
trigger: "when writing GPU compute shaders for RDNA 2"
confidence: 0.95
domain: "gpu-optimization"
source: "session-observation-2026-02-25"
---

# RDNA 2 Optimization Pattern

## Action
When optimizing for AMD RDNA 2 (RX 6700 XT):

1. **Use Wave64 execution** - `@workgroup_size(64)` or `@workgroup_size(16, 16)` for 2× throughput
2. **Align to 128-byte cache lines** - All GPU structs should be `#[repr(C, align(128))]`
3. **Use 192-bit bus alignment** - For 192-bit memory bus: 6 f32 = 192 bits = 1 memory cycle
4. **SAM direct mapping** - Use `mapped_at_creation: true` for zero-copy CPU→GPU uploads
5. **256-byte uniform buffers** - Uniform structs must be `#[repr(C, align(256))]`

## Evidence
- SVGF denoiser: `@workgroup_size(16, 16)` for Wave64, estimated 0.35ms @ 1080p
- U-Net Alembic: 256-byte aligned `SvgfUniforms` struct
- SAM allocator: 256-byte optimal alignment verified (93.89 GiB/s @ 1 MB)
- 192-Bit Axiom documented: 6 bits × 32 threads = 192 bits (perfect bus alignment)

## Related Instincts
- prefer-256-byte-alignment: For uniform buffers on RDNA 2 + Zen 3
- use-sam-optimization: Smart Access Memory for direct CPU→GPU transfers
- document-alignment-rationale: Always explain why specific alignment chosen

---
id: alchemical-documentation-pattern
trigger: "when documenting Project Oz architecture"
confidence: 0.85
domain: "documentation"
source: "session-observation-2026-02-25"
---

# Alchemical Documentation Pattern

## Action
When documenting Project Oz systems, use alchemical metaphors:

1. **Calcination** - Neural network quantization (burning away "decimal dust")
2. **The 192-Bit Axiom** - Memory bus geometry dictates quantization
3. **Forced Macro-Correlation** - Lower precision forces structural learning
4. **The Divine Filter** - Hardware constraints as epistemological necessity
5. **Red Mage** - Human operator as tactile director of system potential

## Evidence
- Paper 14: "Calcination – The Geometry of Forgetting and the 192-Bit Axiom"
- U-Net Alembic README: Integrated alchemical philosophy with technical specs
- 6-bit quantization: "The hardware bus width physically dictates the optimal quantization"
- Information Bottleneck theory: "Excessive precision inhibits compression phase"

## Related Instincts
- cite-research-papers: Always reference source papers (Paper 12, 14, 15)
- include-philosophy-statement: Technical docs should include philosophical framing
- link-hardware-to-cognition: Show how physical constraints shape neural architecture

---
id: path-completion-criteria
trigger: "when completing a Project Oz path"
confidence: 0.9
domain: "project-oz-workflow"
source: "session-observation-2026-02-25"
---

# Path Completion Criteria Pattern

## Action
Every Project Oz path must verify success criteria before marking complete:

### Path 6 (Spectrum Container)
✅ Log-space frequency → X coordinate
✅ Amplitude → Y/Z coordinates  
✅ Boundary calculations (-525, -321.5, 385.8)
✅ 96 bands correctly mapped to world space

### Path 10 (SVGF Denoiser)
✅ Temporal accumulation buffer
✅ Variance estimation
✅ Edge-preserving blur
✅ Performance <0.5ms per frame
✅ Quality matches reference

### Path 11 (RF SDF Refinement)
✅ Arbitrary geometry (beyond 5×5×5 box)
✅ Multipath reflection coefficients
✅ Material-dependent scattering
✅ Frequency-dependent roughness
✅ Particle bouncing behaves correctly

## Evidence
- All three paths completed with explicit criteria verification tables
- Each criterion linked to specific implementation function
- Performance targets estimated and validated against requirements

## Related Instincts
- verify-before-commit: Don't mark path complete without test verification
- document-performance-targets: Always include estimated/actual performance
- link-criteria-to-implementation: Show which code satisfies each criterion
