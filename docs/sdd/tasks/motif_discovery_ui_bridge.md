# Task: Motif Discovery UI Bridge

## Description
Connect the background TimeGNN motif discovery to the Slint UI properties.

## Requirements
- Warp `discover_patterns` to return a result set containing cluster centroids and assignments.
- Use `slint::invoke_from_event_loop` to safely push results into `AppState::learned_patterns`.
- Update the `is-clustering` Slint property to provide visual feedback during processing.
