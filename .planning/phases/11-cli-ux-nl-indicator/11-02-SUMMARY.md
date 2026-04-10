---
phase: 11-cli-ux-nl-indicator
plan: 02
subsystem: cli
tags: [clap, help-output, cli-ux, rust]

requires:
  - phase: 09-binary-rename-version-foundations
    provides: aliast binary name and clap derive CLI struct
provides:
  - Two-tier CLI help with AI setup guidance
  - after_help compact doctor hint on -h
  - after_long_help full env var reference and quick-start examples on --help
affects: []

tech-stack:
  added: []
  patterns: [clap after_help/after_long_help two-tier help pattern]

key-files:
  created: []
  modified: [crates/aliast-daemon/src/main.rs]

key-decisions:
  - "LONG_HELP const string keeps clap attribute clean and readable"
  - "CLI-03 verified as pre-satisfied by Phase 9 CI workflow"

patterns-established:
  - "Two-tier help: after_help for -h compact, after_long_help for --help detailed"

requirements-completed: [CLI-01, CLI-02, CLI-03]

duration: 2min
completed: 2026-04-10
---

# Phase 11 Plan 02: Two-Tier CLI Help with AI Setup Guidance Summary

**Two-tier clap help: -h shows compact doctor hint, --help shows full AI backend env vars and Ollama/Claude quick-start examples**

## Performance

- **Duration:** 2 min
- **Started:** 2026-04-10T23:22:24Z
- **Completed:** 2026-04-10T23:24:10Z
- **Tasks:** 1
- **Files modified:** 1

## Accomplishments
- Added `after_help` attribute for compact `-h` output with "Run `aliast doctor`" hint
- Added `after_long_help` with full AI setup reference: ALIAST_NL_MODEL, ALIAST_NL_BACKEND, ALIAST_ANTHROPIC_KEY, ALIAST_OPENAI_KEY
- Included Ollama and Claude quick-start examples in `--help` output
- Verified CLI-03: `.github/workflows/release.yml` already references `aliast` binary

## Task Commits

Each task was committed atomically:

1. **Task 1: Add two-tier help output with AI setup guidance** - `37d4078` (feat)

**Plan metadata:** pending

## Files Created/Modified
- `crates/aliast-daemon/src/main.rs` - Added LONG_HELP const and clap after_help/after_long_help attributes

## Decisions Made
- Used a `const LONG_HELP: &str` to keep the `#[command(...)]` attribute readable
- CLI-03 confirmed pre-satisfied: release.yml references `aliast` in build/package steps

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- CLI help output complete with AI setup guidance
- NL indicator work (Plan 01) is independent and handled separately

## Self-Check: PASSED

- FOUND: crates/aliast-daemon/src/main.rs
- FOUND: 11-02-SUMMARY.md
- FOUND: commit 37d4078

---
*Phase: 11-cli-ux-nl-indicator*
*Completed: 2026-04-10*
