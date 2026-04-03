---
phase: 02-history-based-suggestions
plan: 01
subsystem: database
tags: [sqlite, rusqlite, zsh-history, ndjson, tdd]

requires:
  - phase: 01-daemon-ipc-ghost-text
    provides: "Protocol message types (Request/Response enums), suggest() placeholder, workspace layout"
provides:
  - "HistoryStore with SQLite WAL mode, prefix search, batch import"
  - "Zsh history parser (plain, extended, multiline formats)"
  - "Record/Ack protocol message variants"
  - "suggest() rewired to HistoryStore-backed prefix lookup returning suffix-only text"
affects: [02-02-daemon-wiring, 02-03-zsh-plugin, 03-nl-suggestions]

tech-stack:
  added: [rusqlite-0.39-bundled, tempfile-3]
  patterns: [sqlite-wal-mode, case-sensitive-like, prepared-cached-statements, transaction-batched-import]

key-files:
  created:
    - crates/alias-core/src/history/mod.rs
    - crates/alias-core/src/history/store.rs
    - crates/alias-core/src/history/parser.rs
    - crates/alias-core/tests/history_store_tests.rs
    - crates/alias-core/tests/history_parser_tests.rs
  modified:
    - crates/alias-core/Cargo.toml
    - crates/alias-core/src/lib.rs
    - crates/alias-protocol/src/message.rs
    - crates/alias-protocol/tests/message_tests.rs
    - crates/alias-core/tests/suggestion_tests.rs

key-decisions:
  - "suggest_prefix returns full command text; caller strips prefix for suffix-only"
  - "HistoryEntry timestamp is Option<i64> to handle plain format (no timestamp)"
  - "SQL wildcard escaping in suggest_prefix prevents injection via % or _ in user input"

patterns-established:
  - "TDD red-green for all new modules: write failing tests then implement"
  - "SQLite PRAGMA initialization pattern: WAL + synchronous=NORMAL + case_sensitive_like=ON"
  - "Transaction-wrapped batch import for history ingestion performance"

requirements-completed: [SUGG-06, SUGG-01]

duration: 3min
completed: 2026-04-03
---

# Phase 2 Plan 01: History Data Layer Summary

**SQLite-backed HistoryStore with WAL mode prefix search, zsh history parser for plain/extended/multiline formats, and Record/Ack protocol messages**

## Performance

- **Duration:** 3 min
- **Started:** 2026-04-03T00:43:01Z
- **Completed:** 2026-04-03T00:46:48Z
- **Tasks:** 2
- **Files modified:** 10

## Accomplishments
- HistoryStore with SQLite WAL mode, case-sensitive LIKE, prefix search, batch import, and SQL wildcard escaping
- Zsh history file parser handling plain, EXTENDED_HISTORY, and multiline backslash-continuation formats
- Record/Ack protocol message variants for command recording via precmd hook
- suggest() rewired from hardcoded placeholder to HistoryStore-backed prefix lookup returning suffix-only text
- 31 total tests passing across alias-core and alias-protocol

## Task Commits

Each task was committed atomically:

1. **Task 1: HistoryStore, history parser, and protocol messages** - `d5007db` (test: RED), `853f40e` (feat: GREEN)
2. **Task 2: Rewire suggest() to use HistoryStore** - `5dc8b0b` (test: RED), `0d13d4d` (feat: GREEN)

_Note: TDD tasks have two commits each (test then feat)_

## Files Created/Modified
- `crates/alias-core/Cargo.toml` - Added rusqlite bundled and tempfile dev-dependency
- `crates/alias-core/src/history/mod.rs` - Public re-exports for HistoryStore, HistoryEntry, parse_history_file
- `crates/alias-core/src/history/store.rs` - SQLite-backed HistoryStore with open, record_command, suggest_prefix, import_entries, count
- `crates/alias-core/src/history/parser.rs` - Zsh history parser with plain, extended, multiline support
- `crates/alias-core/src/lib.rs` - Rewired suggest() to take &HistoryStore, return suffix-only text
- `crates/alias-core/tests/history_store_tests.rs` - 8 tests for prefix search, case sensitivity, wildcards, batch import
- `crates/alias-core/tests/history_parser_tests.rs` - 5 tests for plain, extended, multiline, empty, mixed formats
- `crates/alias-core/tests/suggestion_tests.rs` - 5 tests for HistoryStore-backed suggest with suffix-only returns
- `crates/alias-protocol/src/message.rs` - Added Record request and Ack response variants
- `crates/alias-protocol/tests/message_tests.rs` - 4 new tests for Record/Ack serialization and roundtrip

## Decisions Made
- suggest_prefix returns full command text; the suggest() function strips the prefix to return suffix-only -- cleanly separates DB concern from presentation concern
- HistoryEntry timestamp is Option<i64> to gracefully handle plain zsh history format which has no timestamps
- SQL wildcard characters (% and _) are escaped in suggest_prefix to prevent unintended pattern matching from user input

## Deviations from Plan

None - plan executed exactly as written.

## Known Stubs

None - all functionality is fully wired. Note: alias-daemon connection.rs temporarily fails to compile because it still calls the old suggest(&buf) signature. This is expected and documented in the plan as being fixed in Plan 02-02.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- HistoryStore, parser, and updated suggest() are ready for daemon wiring in Plan 02-02
- Record/Ack protocol messages ready for precmd hook integration
- alias-daemon will need HistoryStore injected into its connection handler (Plan 02-02 scope)

## Self-Check: PASSED

All 8 created/modified files verified on disk. All 4 task commits (d5007db, 853f40e, 5dc8b0b, 0d13d4d) verified in git log.

---
*Phase: 02-history-based-suggestions*
*Completed: 2026-04-03*
