---
phase: 10-daemon-lifecycle
plan: 01
subsystem: daemon
tags: [protocol, serde, ndjson, lifecycle, cancellation-token, atomic-bool]

# Dependency graph
requires:
  - phase: 09-binary-rename-version-foundations
    provides: aliast binary name and workspace version
provides:
  - Shutdown, Enable, Disable, GetStatus request protocol variants
  - ShuttingDown, Status response protocol variants
  - DaemonState struct consolidating store, ai_backend, cancel_token, enabled
  - On, Off, Doctor CLI subcommand stubs
affects: [10-02-stop-on-off-status, 10-03-doctor-autostart]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "DaemonState struct passed through server stack instead of individual params"
    - "AtomicBool for enabled toggle shared across connections"

key-files:
  created: []
  modified:
    - crates/aliast-protocol/src/message.rs
    - crates/aliast-protocol/tests/message_tests.rs
    - crates/aliast-daemon/src/lib.rs
    - crates/aliast-daemon/src/main.rs
    - crates/aliast-daemon/src/server.rs
    - crates/aliast-daemon/src/connection.rs
    - crates/aliast-daemon/tests/e2e_tests.rs
    - crates/aliast-daemon/tests/server_tests.rs

key-decisions:
  - "DaemonState defined in lib.rs (not main.rs) so integration tests can import it"
  - "GetStatus dispatch returns real enabled state from AtomicBool; Shutdown/Enable/Disable are stubs for Plan 02"

patterns-established:
  - "DaemonState: consolidated state struct cloned into each connection handler"
  - "New protocol variants follow existing serde tag=type pattern with rename attributes"

requirements-completed: [LIFE-02, LIFE-03]

# Metrics
duration: 3min
completed: 2026-04-10
---

# Phase 10 Plan 01: Protocol Additions + DaemonState Refactor Summary

**Six lifecycle protocol message variants (Shutdown/Enable/Disable/GetStatus/ShuttingDown/Status) and DaemonState struct consolidating daemon state for the server stack**

## Performance

- **Duration:** 3 min
- **Started:** 2026-04-10T18:56:55Z
- **Completed:** 2026-04-10T18:59:54Z
- **Tasks:** 2
- **Files modified:** 8

## Accomplishments
- Added 4 new Request variants (Shutdown, Enable, Disable, GetStatus) and 2 new Response variants (ShuttingDown, Status) with full NDJSON serialization
- Defined DaemonState struct with store, ai_backend, cancel_token, and enabled (AtomicBool) fields
- Refactored run_server and handle_connection to accept DaemonState instead of 4 separate parameters
- Added On, Off, Doctor CLI subcommand stubs
- All 117 workspace tests pass with zero regressions

## Task Commits

Each task was committed atomically:

1. **Task 1: Protocol additions + serialization tests** - `1a5c595` (feat, TDD)
2. **Task 2: DaemonState struct + refactor signatures + fix tests** - `440049e` (feat)

## Files Created/Modified
- `crates/aliast-protocol/src/message.rs` - Added Shutdown, Enable, Disable, GetStatus request variants and ShuttingDown, Status response variants
- `crates/aliast-protocol/tests/message_tests.rs` - Added 12 serialization/round-trip tests for new variants
- `crates/aliast-daemon/src/lib.rs` - Defined and exported DaemonState struct
- `crates/aliast-daemon/src/main.rs` - Added On/Off/Doctor subcommands, build DaemonState in Start handler
- `crates/aliast-daemon/src/server.rs` - Refactored run_server to accept DaemonState
- `crates/aliast-daemon/src/connection.rs` - Refactored handle_connection and dispatch_request to use DaemonState
- `crates/aliast-daemon/tests/e2e_tests.rs` - Updated spawn_daemon/spawn_daemon_with_ai helpers to construct DaemonState
- `crates/aliast-daemon/tests/server_tests.rs` - Updated start_test_server helper to construct DaemonState

## Decisions Made
- DaemonState defined in lib.rs rather than main.rs so integration tests can import `aliast_daemon::DaemonState`
- GetStatus dispatch reads real AtomicBool enabled state; Shutdown/Enable/Disable return stub responses for Plan 02 to wire

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Protocol variants ready for Plan 02 to wire Shutdown (cancel_token), Enable/Disable (AtomicBool toggle), and enhanced Status
- DaemonState struct carries everything Plan 02/03 need: cancel_token for stop, enabled for toggle, store for doctor
- On/Off/Doctor CLI stubs ready to be implemented in Plans 02 and 03

## Self-Check: PASSED

All 8 modified files verified present. Both task commits (1a5c595, 440049e) verified in git log.

---
*Phase: 10-daemon-lifecycle*
*Completed: 2026-04-10*
