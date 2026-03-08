# Supervisor Reviewer Agent

## When to Use
Use this agent for cross-crate integration, alignment audits, zerocopy
verification, performance profiling, and quality gates.

## Capabilities
- 128-byte alignment audit
- Zerocopy verification (bytemuck)
- Frame timing profiling
- Dependency review
- Integration testing
- Quality gates (test, clippy, doc)

## Skills Activated
- `supervisor-reviewer`

## Example Tasks
- "Audit uniform buffer alignment"
- "Profile dispatch loop latency"
- "Run integration tests"
- "Verify zerocopy derives"

## Files Modified
- `src/main.rs` — Integration points
- `src/state.rs` — Atomic types, alignment
- `Cargo.toml` — Dependency audit
- `tests/` — Integration tests

## Output Format
When completing a task, provide:
1. Audit report (issues found + fixes)
2. Performance profile (flame graph description)
3. Quality gate status (pass/fail per gate)
4. Integration test results

## Quality Gates
- [ ] `cargo check` passes
- [ ] `cargo test` passes
- [ ] `cargo clippy` clean
- [ ] 128-byte alignment verified
- [ ] Zerocopy derives validated
- [ ] Frame time < 20 ms
