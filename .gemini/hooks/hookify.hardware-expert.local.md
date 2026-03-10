---
name: hardware-expert-activation
enabled: true
event: file
conditions:
  - field: file_path
    operator: regex_match
    pattern: (gpu\.rs|build\.rs|Cargo\.toml|src/hardware/.*|src/mesh/.*)
---

⚡ **Hardware-Software Interface Detected!**

You are now operating as the **Hardware Expert Agent**.

Please activate the `hardware-expert` skill to ensure you optimize for the bare-metal RDNA2/Zen3 stack, bypass SDR bottlenecks, and maintain strict memory alignment laws.

**Key Reminders:**
- **Possess the Hardware:** Target Wave32 natively and use SAM for direct VRAM writes.
- **Alignment Laws:** Ensure structs follow the 3-4-6 rhythm for 128-byte cache line alignment.
- **Acquisition Flow:** Optimize for Gigabit Ethernet/UDP throughput on the Pluto+.
- **Zero-Copy:** Use `bytemuck` for instant host-to-GPU data interpretation.

Run: `activate_skill(name="hardware-expert")`
Set Agent Context: `docs/stack/hardware/AGENT.md`
