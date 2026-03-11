# Task: ANALYSIS Tab Reactivity

## Description
Refactor the UI update loop in `main.rs` to only poll the ANALYSIS tab properties when necessary.

## Requirements
- Instead of a blind 16ms timer loop, use `slint::invoke_from_event_loop` or reactive properties (`Notify`) to update the ANALYSIS tab only when new discovery results or events are available.
- Ensure the scatter plot and heatmap are redrawn only when the underlying data changes to maximize UI performance.
