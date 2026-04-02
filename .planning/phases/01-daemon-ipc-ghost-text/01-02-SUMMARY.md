---
phase: 01-daemon-ipc-ghost-text
plan: 02
subsystem: infra
tags: [tokio, unix-socket, ndjson, daemon, serde]

# Dependency graph
requires:
  - phase: 01-01
    provides: "Protocol types (Request/Response) and core suggest() function"
provides:
  - "Unix socket server accepting connections via tokio"
  - "NDJSON request/response handler (ping->pong, complete->suggestion)"
  - "Stale socket cleanup and XDG socket path resolution"
  - "Graceful shutdown on SIGTERM/SIGINT with socket cleanup"
  - "Status command to check if daemon is running"
affects: [01-03-zsh-plugin, daemon-lifecycle, ipc-contract]

# Tech tracking
tech-stack:
  added: [tokio-util CancellationToken, tempfile (dev)]
  patterns: [per-connection tokio::spawn with child CancellationToken, NDJSON line framing with BufReader]

key-files:
  created:
    - crates/alias-daemon/src/lifecycle.rs
    - crates/alias-daemon/src/server.rs
    - crates/alias-daemon/src/connection.rs
    - crates/alias-daemon/src/lib.rs
    - crates/alias-daemon/tests/lifecycle_tests.rs
    - crates/alias-daemon/tests/server_tests.rs
  modified:
    - crates/alias-daemon/src/main.rs
    - crates/alias-daemon/Cargo.toml
    - Cargo.lock

key-decisions:
  - "Used tokio_util::sync::CancellationToken for cooperative shutdown propagation"
  - "BufReader line-based NDJSON framing (one JSON object per line)"
  - "Status command uses sync UnixStream::connect probe (no async needed)"

patterns-established:
  - "CancellationToken hierarchy: server holds parent, each connection gets child_token()"
  - "Connection handler pattern: split stream, BufReader on read half, select! with cancel"
  - "Socket path resolution: XDG_RUNTIME_DIR with /tmp/alias-{uid} fallback"

requirements-completed: [INFRA-01]

# Metrics
duration: 4min
completed: 2026-04-02
---

# Phase 01 Plan 02: Daemon Server Summary

**Unix socket daemon with NDJSON protocol handling, stale socket cleanup, and graceful SIGTERM/SIGINT shutdown**

## Performance

- **Duration:** 4 min
- **Started:** 2026-04-02T21:07:26Z
- **Completed:** 2026-04-02T21:12:04Z
- **Tasks:** 2
- **Files modified:** 9

## Accomplishments
- Lifecycle module with stale socket detection, XDG path resolution, and 0o700 directory permissions
- Socket server accepting connections and spawning per-connection tokio tasks
- NDJSON connection handler dispatching ping and complete requests to protocol responses
- Main.rs wired with signal handling (SIGINT/SIGTERM) and cooperative shutdown via CancellationToken
- 13 tests total: 7 lifecycle + 6 server integration

## Task Commits

Each task was committed atomically:

1. **Task 1: Implement daemon lifecycle management** - `52e034d` (feat)
2. **Task 2: Implement socket server and NDJSON handler** - `97b89bd` (feat)

## Files Created/Modified
- `crates/alias-daemon/src/lifecycle.rs` - Stale socket cleanup, XDG path resolution, socket removal
- `crates/alias-daemon/src/server.rs` - Unix socket accept loop with tokio, cleanup on shutdown
- `crates/alias-daemon/src/connection.rs` - Per-connection NDJSON read/dispatch/write handler
- `crates/alias-daemon/src/lib.rs` - Library crate root exporting public modules
- `crates/alias-daemon/src/main.rs` - CLI wired to server with signal handling and graceful shutdown
- `crates/alias-daemon/Cargo.toml` - Added tempfile dev-dependency
- `crates/alias-daemon/tests/lifecycle_tests.rs` - 7 tests for socket cleanup and path resolution
- `crates/alias-daemon/tests/server_tests.rs` - 6 integration tests for server behavior

## Decisions Made
- Used `tokio_util::sync::CancellationToken` for cooperative shutdown (parent token in server, child tokens per connection)
- Line-based NDJSON framing using `BufReader::read_line` for simplicity and correctness
- Status command uses synchronous `UnixStream::connect` probe since it does not need async

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Rust 2024 edition requires unsafe blocks for env::set_var/remove_var**
- **Found during:** Task 1 (lifecycle tests)
- **Issue:** Tests using `std::env::set_var` and `std::env::remove_var` failed to compile under edition 2024 which treats these as unsafe
- **Fix:** Wrapped env var mutations in `unsafe` blocks with safety comments
- **Files modified:** `crates/alias-daemon/tests/lifecycle_tests.rs`
- **Verification:** All 7 lifecycle tests compile and pass
- **Committed in:** `52e034d` (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (1 blocking)
**Impact on plan:** Trivial adaptation to Rust 2024 edition. No scope creep.

## Issues Encountered
None beyond the deviation above.

## Known Stubs
- `Commands::Stop` in `main.rs:106` prints "not yet implemented" -- intentional per plan, will be implemented when daemon-to-daemon shutdown signaling is added

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Daemon is fully functional: accepts connections, processes requests, shuts down cleanly
- Ready for zsh plugin (01-03) to connect and send NDJSON requests
- `alias-daemon start` can be tested end-to-end with socat or netcat

## Self-Check: PASSED

All 7 created files verified on disk. Both commit hashes (52e034d, 97b89bd) found in git log.

---
*Phase: 01-daemon-ipc-ghost-text*
*Completed: 2026-04-02*
