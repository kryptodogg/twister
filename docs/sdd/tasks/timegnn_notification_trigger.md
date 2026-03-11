# Task: TimeGNN Notification Trigger

## Description
Update `ForensicLogger` in `src/forensic.rs` to provide a reactive trigger for the ML thread.

## Requirements
- When 1000+ new events are appended to the forensic log, signal a background discovery thread.
- Use `crossbeam_channel` or `std::sync::Condvar` for low-latency notification.
- Do not block the main DSP thread.
