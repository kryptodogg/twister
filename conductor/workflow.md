# Development Workflow & Quality Standards

**Project**: Twister v0.5+
**Last Updated**: 2026-03-08

## Development Methodology: Test-Driven Development (TDD)

### Workflow for Every Feature/Fix
1. **Write failing test** (red)
   - Test case for required behavior
   - Minimal test framework setup
   - Run: fails because feature not implemented

2. **Implement feature** (green)
   - Write minimal code to pass test
   - Focus on correctness, not optimization
   - Run: test passes

3. **Refactor** (refactor)
   - Optimize for clarity and performance
   - Add error handling
   - Run: test still passes

4. **Document** (yellow)
   - Add rustdoc comments
   - Update CLAUDE.md if architectural
   - Update conductor/ artifacts

### Minimum Test Requirements
- ✅ 10+ tests per feature before UI wiring
- ✅ Unit tests for algorithms (isolated behavior)
- ✅ Integration tests (components working together)
- ✅ Edge cases (empty input, max size, boundary conditions)
- ✅ Performance tests (timing benchmarks for critical paths)

**Example**: Task 1 (ModularFeatureExtractor) requires:
- 3 unit tests (variable dims, normalization, MSE loss)
- 3 integration tests (with other modules)
- 2 edge case tests (empty batch, max batch size)
- 2 performance tests (inference time, memory)
= 10+ tests minimum

## Git Workflow

### Branching Strategy
```
main (stable, tested, releasable)
  ↓
feature/{phase}-{component}
  ├─ feature/phase-4-timegnn
  ├─ feature/phase-5-pointmamba
  ├─ feature/task-1-modular-features
  └─ feature/fix-training-persistence
```

### Commit Conventions
```
<type>: <subject> (<ticket>)

<body (detailed explanation)>

Co-Authored-By: <person> <email>
```

**Types**:
- `feat`: New feature (creates new capability)
- `fix`: Bug fix (resolves issue)
- `refactor`: Code reorganization (no behavior change)
- `test`: Test additions/changes
- `docs`: Documentation updates
- `perf`: Performance optimization
- `chore`: Build, CI, or tool updates

**Example**:
```
feat: Add ModularFeatureExtractor with FeatureFlags (T.1.1)

Implements dynamic feature dimensionality (196-381D) based on 
learned FeatureFlags. Includes reconstruction loss for anomaly 
scoring. 10 tests covering variable dims, batch sizes, and edge cases.

Co-Authored-By: Jules <email>
```

### Pull Request Process
1. **Branch created** from main
2. **TDD implementation** (red → green → refactor → doc)
3. **Push to remote** with clear commit messages
4. **Create PR** with:
   - Linked ticket (if applicable)
   - Test results (passing/failing)
   - Performance impact (if any)
5. **Code review** (peer review, architecture check)
6. **Merge to main** (squash or rebase, not merge commits)

## Code Quality Gates

### Before Merging to Main

✅ **Compilation**
- `cargo build` succeeds with < 100 warnings
- No clippy errors (`cargo clippy`)

✅ **Tests**
- All tests pass (`cargo test`)
- New feature has 10+ tests
- Integration tests pass
- Coverage: aim for 80%+ on new code

✅ **Documentation**
- rustdoc comments on public items
- CLAUDE.md updated if architectural
- conductor/ artifacts updated if plans changed

✅ **Performance**
- No regressions in critical paths
- Memory usage stable (<2GB for training)
- UI remains responsive (>30 fps)

✅ **Security**
- No unsafe code without justification
- Dependencies reviewed (no zero-day vulnerabilities)
- Forensic logging preserves privacy (no raw audio storage)

### Definition of Done (Feature)

A feature is DONE when:
- ✅ Code written and tested (TDD cycle complete)
- ✅ 10+ tests passing
- ✅ Documentation updated
- ✅ Merged to main
- ✅ Performance verified
- ✅ Next task (UI wiring or dependent feature) can proceed

## Review Checklist (Code Reviewer)

- [ ] Tests cover main behavior + edge cases
- [ ] Code matches project style (see code styleguides/)
- [ ] No unsafe code without comment
- [ ] Error handling present and tested
- [ ] Performance acceptable (no obvious inefficiencies)
- [ ] Documentation complete and accurate
- [ ] Commit messages clear and linked to tickets
- [ ] No secrets/credentials in code or logs

## Testing Standards

### Unit Tests
Location: `src/{module}.rs` (inline with `#[cfg(test)]` modules)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_basic_behavior() {
        // Arrange: set up inputs
        let input = vec![1.0, 2.0, 3.0];
        
        // Act: call function
        let result = compute(input);
        
        // Assert: verify output
        assert_eq!(result, expected);
    }
}
```

### Integration Tests
Location: `tests/{module}_integration.rs`

```
tests/
├─ ml_feature_encoder_integration.rs
├─ timegnn_training_integration.rs
└─ point_mamba_integration.rs
```

### Performance Tests
Location: `benches/{module}.rs` (using criterion crate)

```rust
#[bench]
fn bench_encoder_inference(b: &mut Bencher) {
    let encoder = ModularFeatureEncoder::new(...);
    b.iter(|| encoder.forward(&input));
}
```

## Release & Deployment

### Version Numbering
- v0.5 (Phase 3 complete)
- v0.5.1 (Phase 4 complete)
- v0.5.2 (Phase 5 complete)
- v0.6 (Real-time pattern matching)

### Release Checklist
- [ ] All tests passing
- [ ] CHANGELOG.md updated
- [ ] Version bumped in Cargo.toml
- [ ] Tag created (git tag v0.5.1)
- [ ] Build artifacts generated
- [ ] Documentation current

## IDE Configuration

### VS Code Recommended Extensions
- rust-analyzer
- Even Better TOML
- CodeLLDB (debugging)
- Slint (UI language support)

### Recommended Settings
```json
{
  "[rust]": {
    "editor.formatOnSave": true,
    "editor.defaultFormatter": "rust-lang.rust-analyzer"
  },
  "rust-analyzer.checkOnSave.command": "clippy"
}
```

## Daily Standup Questions

**For Implementation Team**:
- What did I complete yesterday?
- What am I working on today?
- Am I blocked (anything needing escalation)?
- Do I need code review?

**For Project Manager**:
- Any 🔴 blockers?
- Track any off-schedule tasks?
- Update tracks.md weekly?
- Anything requiring schedule adjustment?

## Continuous Integration (Aspirational)

**Future Setup** (post-Phase 4):
- GitHub Actions: Run `cargo test` on PR
- Automated code coverage reporting
- Performance regression detection
- Automated documentation generation

**Current**: Manual testing before merge

---

See index.md for current task assignments and timeline.
