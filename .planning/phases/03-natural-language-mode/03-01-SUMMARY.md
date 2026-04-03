---
phase: 03-natural-language-mode
plan: 01
subsystem: ai
tags: [ollama, async-trait, reqwest, ndjson, ai-backend]

requires:
  - phase: 01-daemon-ipc-ghost-text
    provides: NDJSON protocol types (Request/Response enums), daemon connection handler
provides:
  - AiBackend async trait for pluggable AI backends
  - OllamaBackend implementation using reqwest to localhost Ollama
  - Generate request and Command response NDJSON protocol variants
affects: [03-natural-language-mode, 04-intelligence-layer]

tech-stack:
  added: [async-trait, reqwest, thiserror]
  patterns: [async trait with object safety, error enum with thiserror, Ollama /api/chat integration]

key-files:
  created:
    - crates/alias-core/src/ai/mod.rs
    - crates/alias-core/src/ai/ollama.rs
    - crates/alias-core/tests/ai_tests.rs
  modified:
    - crates/alias-protocol/src/message.rs
    - crates/alias-protocol/tests/message_tests.rs
    - crates/alias-core/src/lib.rs
    - crates/alias-core/Cargo.toml
    - crates/alias-daemon/src/connection.rs

key-decisions:
  - "Kept AiBackend trait and OllamaBackend in alias-core (not split across crates) for simplicity in Phase 3"
  - "Used reqwest directly for Ollama instead of genai crate -- simpler for single-backend Phase 3"
  - "60-second timeout on generate, 3-second timeout on health_check"

patterns-established:
  - "AiBackend trait pattern: async generate() + health_check() + name() with AiError enum"
  - "Ollama integration: POST /api/chat with stream:false, system prompt for command-only output"

requirements-completed: [NL-07, NL-05, NL-02]

duration: 3min
completed: 2026-04-03
---

# Phase 3 Plan 01: AI Backend Trait and OllamaBackend Summary

**AiBackend async trait with OllamaBackend reqwest implementation and Generate/Command NDJSON protocol types**

## Performance

- **Duration:** 3 min
- **Started:** 2026-04-03T02:34:02Z
- **Completed:** 2026-04-03T02:37:00Z
- **Tasks:** 1
- **Files modified:** 9

## Accomplishments
- Defined AiBackend async trait with generate(), health_check(), and name() methods
- Implemented OllamaBackend that POSTs to /api/chat with stream:false and a system prompt constraining output to shell commands only
- Added Generate request and Command response variants to the NDJSON protocol
- Full test coverage: 6 new protocol tests + 5 AI backend tests, all 42 workspace tests pass

## Task Commits

Each task was committed atomically:

1. **Task 1: Protocol types + AI backend trait + OllamaBackend** - `47f3cec` (feat)

## Files Created/Modified
- `crates/alias-core/src/ai/mod.rs` - AiBackend trait definition with AiError enum
- `crates/alias-core/src/ai/ollama.rs` - OllamaBackend implementation with reqwest HTTP client
- `crates/alias-core/tests/ai_tests.rs` - Unit tests for trait object safety, constructor, health_check, and generate error handling
- `crates/alias-protocol/src/message.rs` - Added Generate request and Command response variants
- `crates/alias-protocol/tests/message_tests.rs` - Serialization/deserialization tests for new variants
- `crates/alias-core/src/lib.rs` - Wired pub mod ai
- `crates/alias-core/Cargo.toml` - Added async-trait, reqwest, thiserror, tokio, serde dependencies
- `crates/alias-daemon/src/connection.rs` - Added placeholder Generate handler for match exhaustiveness
- `Cargo.lock` - Updated lockfile with new dependencies

## Decisions Made
- Kept AiBackend trait and OllamaBackend in alias-core rather than splitting across crates -- simpler for Phase 3, can refactor in Phase 5 if needed
- Used reqwest directly instead of genai crate for Ollama -- single backend keeps dependencies minimal
- 60-second timeout for generate requests (Ollama cold starts), 3-second timeout for health checks

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Added placeholder Generate handler in daemon dispatch**
- **Found during:** Task 1 (compilation)
- **Issue:** Adding Generate variant to Request enum caused non-exhaustive match in daemon's dispatch_request
- **Fix:** Added placeholder arm returning Error response with "generate not yet implemented" message
- **Files modified:** crates/alias-daemon/src/connection.rs
- **Verification:** cargo test --workspace passes
- **Committed in:** 47f3cec (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (1 blocking)
**Impact on plan:** Necessary for compilation. The placeholder will be replaced when Plan 03-02 wires the AI backend into the daemon.

## Issues Encountered
None

## Known Stubs

- `crates/alias-daemon/src/connection.rs` line ~95: Generate request handler returns Error "generate not yet implemented" -- intentional placeholder, will be wired in Plan 03-02

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- AiBackend trait and OllamaBackend ready for daemon wiring in Plan 03-02
- Generate/Command protocol types ready for zsh plugin integration in Plan 03-03
- No blockers for downstream plans

---
*Phase: 03-natural-language-mode*
*Completed: 2026-04-03*

## Self-Check: PASSED
- All created files exist on disk
- Commit 47f3cec verified in git log
- All 42 workspace tests pass (19 protocol + 18 core + 5 AI)
