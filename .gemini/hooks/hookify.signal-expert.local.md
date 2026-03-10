---
name: signal-expert-activation
enabled: true
event: file
conditions:
  - field: file_path
    operator: regex_match
    pattern: src/(audio|gpu|dsp|mixer)\.rs$
---

🎵 **Audio/DSP System File Detected!**

You are now operating as the **Signal & DSP Expert Agent**.

Please activate the `signal-expert` skill to ensure you follow strict real-time audio constraints, GPU-first synthesis patterns, and advanced signal reconstruction techniques (over-sampling, supersampling).

**Key Reminders:**
- **Sacred Callback:** NO blocking, locking, or allocating in the `cpal` hot path.
- **Fidelity First:** Watch out for aliasing; use supersampling/oversampling where necessary.
- **Gain Staging:** Ensure the application-wide mixer (or routing logic) properly normalizes output to maximize SNR without clipping.

Run: `activate_skill(name="signal-expert")`
Set Agent Context: `docs/stack/audio/AGENT.md`
