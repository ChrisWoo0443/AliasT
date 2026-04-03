---
phase: 05-cloud-backends-terminal-compatibility
plan: 01
subsystem: ai
tags: [claude, openai, anthropic, reqwest, async-trait, ai-backend]

requires:
  - phase: 03-natural-language-mode
    provides: AiBackend trait, OllamaBackend, SYSTEM_PROMPT, daemon AI dispatch
provides:
  - ClaudeBackend implementing AiBackend for Anthropic Messages API
  - OpenAiBackend implementing AiBackend for OpenAI Chat Completions API
  - ALIAS_NL_BACKEND env var dispatch in daemon (claude/openai/ollama)
  - Graceful degradation when API keys are missing
affects: [05-cloud-backends-terminal-compatibility]

tech-stack:
  added: []
  patterns: [cloud-api-backend-pattern, env-var-backend-dispatch]

key-files:
  created:
    - crates/alias-core/src/ai/claude.rs
    - crates/alias-core/src/ai/openai.rs
  modified:
    - crates/alias-core/src/ai/mod.rs
    - crates/alias-core/tests/ai_tests.rs
    - crates/alias-daemon/src/main.rs

key-decisions:
  - "Reuse SYSTEM_PROMPT from ollama module for all backends (single source of truth)"
  - "with_base_url constructor on both backends enables testability without mocks"
  - "Unknown ALIAS_NL_BACKEND values default to ollama for backward compatibility"

patterns-established:
  - "Cloud backend pattern: struct with client/api_key/base_url/model, with_base_url for testing"
  - "Graceful API key check: missing key logs warning and disables NL mode, never crashes"

requirements-completed: [NL-06]

duration: 2min
completed: 2026-04-03
---

# Phase 05 Plan 01: Cloud AI Backends Summary

**Claude and OpenAI cloud backends implementing AiBackend trait with env-var-based dispatch in daemon**

## Performance

- **Duration:** 2 min
- **Started:** 2026-04-03T19:47:45Z
- **Completed:** 2026-04-03T19:50:19Z
- **Tasks:** 2
- **Files modified:** 5

## Accomplishments
- ClaudeBackend sends requests to Anthropic Messages API with x-api-key and anthropic-version headers
- OpenAiBackend sends requests to OpenAI Chat Completions API with Bearer token authorization
- Daemon dispatches to correct backend via ALIAS_NL_BACKEND env var (defaults to ollama)
- Missing API keys gracefully disable NL mode with a warning log instead of crashing
- 8 new tests covering both backends (name, object safety, unreachable server errors)

## Task Commits

Each task was committed atomically:

1. **Task 1: ClaudeBackend and OpenAiBackend implementations with tests** - `e005857` (feat)
2. **Task 2: Wire backend selection in daemon main.rs** - `97a57f6` (feat)

_Note: TDD task 1 had RED commit `00bf921` (test) before GREEN commit `e005857` (feat)_

## Files Created/Modified
- `crates/alias-core/src/ai/claude.rs` - ClaudeBackend struct implementing AiBackend for Anthropic API
- `crates/alias-core/src/ai/openai.rs` - OpenAiBackend struct implementing AiBackend for OpenAI API
- `crates/alias-core/src/ai/mod.rs` - Added pub mod claude and pub mod openai declarations
- `crates/alias-core/tests/ai_tests.rs` - 8 new tests for Claude and OpenAI backends
- `crates/alias-daemon/src/main.rs` - ALIAS_NL_BACKEND dispatch with API key validation

## Decisions Made
- Reused SYSTEM_PROMPT from ollama module rather than duplicating it in each backend
- Used with_base_url constructors on both backends for test isolation without HTTP mocking
- Unknown ALIAS_NL_BACKEND values fall through to ollama for backward compatibility
- No model validation (any string accepted, per plan decision D-03)

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

**External services require manual configuration:**
- `ALIAS_ANTHROPIC_KEY` - API key from https://console.anthropic.com -> API Keys
- `ALIAS_OPENAI_KEY` - API key from https://platform.openai.com/api-keys
- `ALIAS_NL_BACKEND` - Set to "claude" or "openai" to use cloud backends (defaults to "ollama")

## Next Phase Readiness
- Cloud backends ready for use, pending user API key configuration
- Terminal compatibility plan (05-02) can proceed independently

## Self-Check: PASSED

All 6 files verified present. All 3 commits verified in git log.

---
*Phase: 05-cloud-backends-terminal-compatibility*
*Completed: 2026-04-03*
