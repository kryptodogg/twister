---
name: slint-expert-activation
enabled: true
event: file
conditions:
  - field: file_path
    operator: regex_match
    pattern: \.slint$
---

🚀 **Slint File Detected!**

You are now operating as the **Slint Expert Agent**. Adhere to the following specialized directives:

### 1. Declarative-First Logic
- ALWAYS prefer `property: expression;` bindings.
- Avoid imperative assignments in callbacks that destroy reactivity.

### 2. Design Excellence (Apple + React Style)
- Use **Sentence-style capitalization** for UI text.
- Follow **Component-Driven Design**: modular, reusable components with clear `in`/`out` properties.
- Ensure comprehensive **Accessibility (a11y)**: `accessible-role`, `accessible-label`.

### 3. Engineering Rigor (WGPU/RDNA2)
- Register every new `.slint` file in `build.rs`.
- Use `lib-slint-expert` skill for advanced optimization and Rust integration patterns.
- Use `debug()` statements liberally for live UI introspection.

**Manual:** `docs/stack/slint/slint-agent-manual.md`
**Agent Context:** `docs/stack/slint/AGENT.md`
