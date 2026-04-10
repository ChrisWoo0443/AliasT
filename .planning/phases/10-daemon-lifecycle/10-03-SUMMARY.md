---
phase: 10-daemon-lifecycle
plan: 03
subsystem: cli, daemon, plugin
tags: [rust, zsh, doctor, diagnostics, auto-start, unix-socket]

# Dependency graph
requires:
  - phase: 10-01
    provides: DaemonState struct, Commands enum with Doctor stub, lifecycle module
  - phase: 10-02
    provides: stop/on/off/status subcommands, protocol variants
provides:
  - "aliast doctor subcommand with 6 diagnostic checks"
  - "DoctorCheck struct for structured health reporting"
  - "Plugin auto-start with bounded 500ms retry in _aliast_connect"
  - "Centralized daemon spawn logic (single spawn location)"
affects: [future-diagnostics, user-onboarding, plugin-reliability]

# Tech tracking
tech-stack:
  added: [reqwest (aliast-daemon dependency for Ollama HTTP health check)]
  patterns: [testable _at() variants for filesystem/socket checks, TDD for diagnostic modules]

key-files:
  created:
    - crates/aliast-daemon/src/doctor.rs
    - crates/aliast-daemon/tests/doctor_tests.rs
  modified:
    - crates/aliast-daemon/src/main.rs
    - crates/aliast-daemon/src/lib.rs
    - crates/aliast-daemon/Cargo.toml
    - plugin/aliast.plugin.zsh

key-decisions:
  - "Added check_daemon_running_at() and check_history_db_at() testable variants to avoid coupling tests to live system state"
  - "Doctor test for daemon_running validates structure rather than asserting daemon is down, since daemon may be running in dev"

patterns-established:
  - "Testable _at() variants: public functions that accept path params for test isolation, production wrappers compute paths internally"
  - "DoctorCheck struct pattern: name, passed, detail, fix fields for uniform diagnostic reporting"

requirements-completed: [LIFE-01, LIFE-04]

# Metrics
duration: 4min
completed: 2026-04-10
---

# Phase 10 Plan 03: Doctor Diagnostics and Plugin Auto-Start Summary

**aliast doctor with 6 health checks (daemon, AI backend, API key, Ollama, key validity, history DB) and plugin auto-start with bounded 500ms retry**

## Performance

- **Duration:** 4 min
- **Started:** 2026-04-10T19:10:49Z
- **Completed:** 2026-04-10T19:15:28Z
- **Tasks:** 2
- **Files modified:** 7

## Accomplishments
- Doctor module with 6 independent diagnostic checks, each reporting pass/fail with actionable fix instructions
- 10 unit tests covering all synchronous check functions (TDD workflow: RED then GREEN)
- Plugin _aliast_connect auto-starts daemon with bounded retry (50ms x 10 = 500ms max), PATH check prevents delay when binary missing
- Centralized daemon spawn in _aliast_connect -- _aliast_reconnect delegates instead of duplicating spawn logic

## Task Commits

Each task was committed atomically:

1. **Task 1: Doctor module with unit tests and 6 diagnostic checks** - `5d7a618` (feat, TDD)
2. **Task 2: Plugin auto-start with bounded retry in _aliast_connect** - `70e0901` (feat)

_Note: Task 1 followed TDD -- tests written first (RED), then implementation (GREEN)._

## Files Created/Modified
- `crates/aliast-daemon/src/doctor.rs` - 6 diagnostic checks with DoctorCheck struct, run_doctor_checks(), print_doctor_report()
- `crates/aliast-daemon/tests/doctor_tests.rs` - 10 unit tests for sync check functions
- `crates/aliast-daemon/src/main.rs` - Doctor subcommand wired in Commands enum and match handler
- `crates/aliast-daemon/src/lib.rs` - Registered doctor module as public
- `crates/aliast-daemon/Cargo.toml` - Added reqwest dependency for Ollama HTTP health check
- `plugin/aliast.plugin.zsh` - _aliast_connect with spawn+bounded-retry, simplified _aliast_reconnect
- `Cargo.lock` - Updated with reqwest dependency resolution

## Decisions Made
- Added `check_daemon_running_at()` testable variant -- the live `check_daemon_running()` probes the real socket which may succeed in dev environments, so tests use a non-existent path for deterministic failure testing
- Added `check_history_db_at()` testable variant (per plan) -- avoids coupling to user filesystem
- Doctor test validates DoctorCheck structure regardless of daemon state, plus separate test with bogus socket path ensures failure path works

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Made check_daemon_running testable with _at() variant**
- **Found during:** Task 1 (doctor tests)
- **Issue:** Plan's test assumed daemon is not running, but daemon IS running on dev machine causing test_check_daemon_running_no_socket to fail
- **Fix:** Added `check_daemon_running_at(path)` variant; test uses non-existent socket path for deterministic failure; separate test validates structure of live check
- **Files modified:** crates/aliast-daemon/src/doctor.rs, crates/aliast-daemon/tests/doctor_tests.rs
- **Verification:** All 10 tests pass regardless of daemon state
- **Committed in:** 5d7a618 (Task 1 commit)

**2. [Rule 3 - Blocking] Wrapped set_var/remove_var in unsafe blocks for Rust 2024 edition**
- **Found during:** Task 1 (doctor tests)
- **Issue:** Rust 2024 edition makes std::env::set_var and remove_var unsafe; plan's test code didn't include unsafe blocks
- **Fix:** Wrapped all env var manipulations in unsafe blocks following existing lifecycle_tests.rs pattern
- **Files modified:** crates/aliast-daemon/tests/doctor_tests.rs
- **Verification:** Compilation succeeds, all tests pass
- **Committed in:** 5d7a618 (Task 1 commit)

---

**Total deviations:** 2 auto-fixed (1 bug, 1 blocking)
**Impact on plan:** Both fixes necessary for test correctness. No scope creep. Testable _at() variant is a strict improvement.

## Issues Encountered
None beyond the auto-fixed deviations above.

## User Setup Required
None - no external service configuration required.

## Known Stubs
None - all functionality is fully wired.

## Next Phase Readiness
- All LIFE requirements (LIFE-01 through LIFE-04) now have implementations
- Doctor subcommand ready for manual testing: `aliast doctor`
- Plugin auto-start ready for integration testing in fresh terminal sessions

## Self-Check: PASSED

All created files verified present. Both task commits verified in git log.

---
*Phase: 10-daemon-lifecycle*
*Completed: 2026-04-10*
