---
phase: 10-daemon-lifecycle
plan: "02"
subsystem: daemon-lifecycle
tags: [shutdown, toggle, cli, ipc, gating]
dependency_graph:
  requires: [10-01]
  provides: [stop-subcommand, on-off-toggle, status-enhanced, enabled-check-gating]
  affects: [connection-handler, main-cli]
tech_stack:
  added: []
  patterns: [inline-shutdown-before-cancel, sync-ipc-helper, atomic-bool-gating]
key_files:
  created: []
  modified:
    - crates/aliast-daemon/src/connection.rs
    - crates/aliast-daemon/src/main.rs
    - crates/aliast-daemon/tests/e2e_tests.rs
decisions:
  - "Shutdown handled inline in handle_connection, not via dispatch_request, to guarantee ShuttingDown response is flushed before cancel_token.cancel()"
  - "send_ipc_request helper uses sync std::os::unix::net::UnixStream with 5s read timeout for stop/on/off/status subcommands"
  - "Status subcommand uses serde_json::Value parsing to extract version and enabled state from GetStatus response"
metrics:
  duration: 4min
  completed: "2026-04-10"
---

# Phase 10 Plan 02: Lifecycle Dispatch + CLI Subcommands Summary

Wire Shutdown/Enable/Disable/GetStatus dispatch handlers, enabled-check gating on Complete/Generate, and stop/on/off/status CLI subcommands via sync IPC helper.

## What was done

### Task 1: Dispatch handlers + enabled-check gating (TDD)
- **RED:** Added 5 failing integration tests: `test_shutdown_via_protocol`, `test_enable_disable_toggle`, `test_disabled_complete_returns_empty`, `test_disabled_generate_returns_error`, `test_disabled_record_still_works`
- **GREEN:** Implemented shutdown inline in `handle_connection` (writes ShuttingDown response, flushes, then cancels root token). Wired Enable/Disable to store/load on `DaemonState.enabled` AtomicBool with `Ordering::Relaxed`. Added gating: Complete returns empty suggestion when disabled, Generate returns "aliast is paused" error when disabled, Record bypasses gating per D-05.
- **Commit:** `02c5543` (RED), `2728080` (GREEN)

### Task 2: CLI subcommands via sync IPC
- Added `send_ipc_request` helper function using sync `std::os::unix::net::UnixStream` with 5-second read timeout
- `aliast stop`: sends `{"type":"shutdown"}`, prints "daemon stopped" on success
- `aliast on`: sends `{"type":"enable"}`, prints "suggestions enabled" on success
- `aliast off`: sends `{"type":"disable"}`, prints "suggestions disabled" on success
- `aliast status`: sends `{"type":"get_status"}`, shows version, enabled/disabled state, and socket path
- All subcommands exit with code 1 when daemon is not running
- **Commit:** `5330e01`

## Deviations from Plan

None -- plan executed exactly as written.

## Verification

- `cargo test --workspace` passes (all tests green)
- `cargo build -p aliast-daemon` succeeds
- 5 `Ordering::Relaxed` uses in connection.rs (2 gating + enable store + disable store + get_status load)
- `cancel_token.cancel()` in connection.rs shutdown handler
- `ShuttingDown` response written before cancel in handle_connection
- "aliast is paused" error message for disabled Generate
- 17 e2e integration tests pass (12 existing + 5 new lifecycle tests)

## Known Stubs

None -- all lifecycle stubs from Plan 01 have been replaced with working implementations. The Doctor subcommand stub remains (out of scope for this plan, addressed in Plan 03).

## Self-Check: PASSED
