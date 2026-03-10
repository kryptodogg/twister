---
name: prevent-dead-code
enabled: true
event: stop
pattern: .*
---

🛑 **Final Verification: No Dead Code or Warnings Allowed**

The project mandate is "No dead code warnings." Before you finish, you must ensure that the latest build is completely clean.

**Verification Checklist:**
- [ ] Run `cargo check` or `cargo build` and verify **ZERO** warnings in the output.
- [ ] If warnings exist (e.g., `unused_field`, `dead_code`), you must fix them before stopping.
- [ ] Ensure all new features are properly integrated and utilized.

**Mandate:** Clean code is a requirement for completion.
