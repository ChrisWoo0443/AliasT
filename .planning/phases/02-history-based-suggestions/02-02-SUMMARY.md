---
phase: 02-history-based-suggestions
plan: 02
subsystem: daemon
tags: [sqlite, arc-mutex, daemon-wiring, history-store, e2e-tests]

requires:
  - phase: 02-history-based-suggestions
    plan: 01
    provides: "HistoryStore, parse_history_file, suggest(store, buffer), Record/Ack protocol variants"
provides:
  - "Daemon initializes SQLite at ~/.local/share/alias/history.db on startup"
  - "Auto-imports ~/.zsh_history when database is empty"
  - "Complete requests return history-based prefix suggestions via HistoryStore"
  - "Record requests store commands and return Ack"
  - "Shared Arc<Mutex<HistoryStore>> passed through server to connection handlers"
affects: [02-03-zsh-plugin, 03-nl-suggestions]

tech-stack:
  added: []
  patterns: [arc-mutex-shared-state, sync-mutex-for-sqlite, auto-import-on-empty-db]

key-files:
  created: []
  modified:
    - crates/alias-daemon/src/main.rs
    - crates/alias-daemon/src/server.rs
    - crates/alias-daemon/src/connection.rs
    - crates/alias-daemon/tests/e2e_tests.rs
    - crates/alias-daemon/tests/server_tests.rs

key-decisions:
  - "std::sync::Mutex (not tokio::sync::Mutex) for HistoryStore since SQLite ops are microsecond-fast synchronous calls"
  - "Synthetic timestamps (line index + 1) assigned to history entries with no timestamp during auto-import"

patterns-established:
  - "Arc<Mutex<HistoryStore>> shared state pattern for thread-safe SQLite access"
  - "Auto-import on empty DB: check count() == 0, parse, import -- idempotent one-time operation"

requirements-completed: [SUGG-01, SUGG-06]

duration: 3min
completed: 2026-04-03
---

# Phase 2 Plan 02: Daemon HistoryStore Wiring Summary

**SQLite history store wired into daemon startup with auto-import, shared via Arc<Mutex> to connection handlers for prefix-matched suggestions and command recording**

## Performance

- **Duration:** 3 min
- **Started:** 2026-04-03T00:49:20Z
- **Completed:** 2026-04-03T00:52:55Z
- **Tasks:** 2
- **Files modified:** 5

## Accomplishments
- Daemon initializes SQLite database at ~/.local/share/alias/history.db on startup
- Auto-imports ~/.zsh_history entries when database is empty, with synthetic timestamps for plain-format entries
- Complete requests dispatch through HistoryStore-backed suggest() returning suffix-only text
- Record requests store commands with current Unix timestamp and return Ack
- All 49 workspace tests pass including 5 new E2E tests and 6 updated server tests

## Task Commits

Each task was committed atomically:

1. **Task 1: Wire HistoryStore into daemon startup and connection handler** - `a10699d` (feat)
2. **Task 2: Update E2E and server tests for HistoryStore wiring** - `c5f2f6c` (test)

## Files Created/Modified
- `crates/alias-daemon/src/main.rs` - HistoryStore::open, auto-import zsh history, Arc<Mutex> wrapping
- `crates/alias-daemon/src/server.rs` - Accepts and passes Arc<Mutex<HistoryStore>> to connection handlers
- `crates/alias-daemon/src/connection.rs` - Dispatches Complete via suggest(), Record via record_command()
- `crates/alias-daemon/tests/e2e_tests.rs` - 5 tests: record/ack, prefix suggestion, empty DB, ping/pong, multi-client
- `crates/alias-daemon/tests/server_tests.rs` - Updated spawn helper with HistoryStore, tests record before complete

## Decisions Made
- Used std::sync::Mutex (not tokio::sync::Mutex) since SQLite operations are synchronous and complete in microseconds -- avoids async overhead
- Assigned synthetic timestamps (line index + 1) to history entries with no timestamp during auto-import to preserve ordering

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Updated server_tests.rs for new run_server signature**
- **Found during:** Task 2 (E2E test execution)
- **Issue:** server_tests.rs (from Phase 01) still called run_server with 2 args instead of 3
- **Fix:** Updated start_test_server() to create HistoryStore and pass to run_server; updated tests that expected hardcoded suggestions to record data first
- **Files modified:** crates/alias-daemon/tests/server_tests.rs
- **Verification:** All 6 server tests pass
- **Committed in:** c5f2f6c (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (1 blocking)
**Impact on plan:** Necessary to fix pre-existing tests broken by the new API. No scope creep.

## Known Stubs

None - all functionality is fully wired with real data flow.

## Issues Encountered
None.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Daemon fully wired with HistoryStore: serves real suggestions and records commands
- Ready for zsh plugin integration (Plan 02-03) to wire precmd hook for recording
- Ready for Phase 03 NL suggestions to add alongside history-based suggestions

## Self-Check: PASSED

All 5 modified files verified on disk. Both task commits (a10699d, c5f2f6c) verified in git log.

---
*Phase: 02-history-based-suggestions*
*Completed: 2026-04-03*
