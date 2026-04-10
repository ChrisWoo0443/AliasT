# Roadmap: AliasT

## Milestones

- ✅ **v1.0 MVP** — Phases 1-5 (shipped 2026-04-03)
- ✅ **v1.1 AliasT + Homebrew Distribution** — Phases 6-8 (shipped 2026-04-10)
- 🚧 **v1.2 CLI Polish & Reliability** — Phases 9-11 (in progress)

## Phases

<details>
<summary>✅ v1.0 MVP (Phases 1-5) — SHIPPED 2026-04-03</summary>

- [x] **Phase 1: Daemon + IPC + Ghost Text** — 3/3 plans, completed 2026-04-02
- [x] **Phase 2: History-Based Suggestions** — 3/3 plans, completed 2026-04-03
- [x] **Phase 3: Natural Language Mode** — 3/3 plans, completed 2026-04-03
- [x] **Phase 4: Context & Ranking Intelligence** — 2/2 plans, completed 2026-04-03
- [x] **Phase 5: Cloud Backends & Terminal Compat** — 2/2 plans, completed 2026-04-03

</details>

<details>
<summary>✅ v1.1 AliasT + Homebrew Distribution (Phases 6-8) — SHIPPED 2026-04-10</summary>

- [x] **Phase 6: Project Rename** — 2/2 plans, completed 2026-04-03
- [x] **Phase 7: CI/CD Release Pipeline** — 1/1 plan, completed 2026-04-09
- [x] **Phase 8: Homebrew Tap + Formula** — 2/2 plans, completed 2026-04-10

</details>

### v1.2 CLI Polish & Reliability (In Progress)

**Milestone Goal:** Fix daemon lifecycle bugs, rename binary to `aliast`, add AI setup guidance, and polish the NL mode indicator.

- [ ] **Phase 9: Binary Rename + Version Foundations** - Rename binary from `aliast-daemon` to `aliast` and unify workspace versions
- [x] **Phase 10: Daemon Lifecycle** - Complete daemon start/stop/toggle/doctor control via CLI (completed 2026-04-10)
- [ ] **Phase 11: CLI UX + NL Indicator** - Polish help output, add AI setup guidance, and replace NL text indicator with colored dot

## Phase Details

### Phase 9: Binary Rename + Version Foundations
**Goal**: Users interact with a single `aliast` binary that reports the correct version
**Depends on**: Phase 8
**Requirements**: BIN-01, BIN-02, BIN-03
**Success Criteria** (what must be TRUE):
  1. Running `aliast --version` outputs the current workspace version (not `0.1.0`)
  2. The installed binary is named `aliast` (not `aliast-daemon`)
  3. All workspace crates inherit version from root Cargo.toml (no per-crate hardcoded versions)
  4. Existing tests pass without hardcoded version strings
**Plans**: 3 plans

Plans:
- [x] 09-01-PLAN.md — Workspace version unification (version 1.2.0 + test assertion updates)
- [x] 09-02-PLAN.md — Binary rename to `aliast` (Cargo [[bin]] + source/plugin/CI/docs updates)
- [x] 09-03-PLAN.md — Homebrew formula update for `aliast` binary name

### Phase 10: Daemon Lifecycle
**Goal**: Users have complete CLI control over the daemon -- start, stop, toggle, health check -- and fresh shells auto-connect without manual intervention
**Depends on**: Phase 9
**Requirements**: LIFE-01, LIFE-02, LIFE-03, LIFE-04
**Success Criteria** (what must be TRUE):
  1. Opening a new terminal session produces ghost-text suggestions without the user manually starting the daemon
  2. Running `aliast stop` gracefully shuts down a running daemon
  3. Running `aliast off` disables suggestions across all terminal sessions; `aliast on` re-enables them
  4. Running `aliast doctor` shows daemon status, AI backend configuration, and actionable fix instructions for any issues found
  5. Running `aliast status` shows whether the daemon is running, the socket path, and configured AI backend
**Plans**: 4 plans

Plans:
- [x] 10-01-PLAN.md — Protocol additions (Shutdown/Enable/Disable/GetStatus) + DaemonState refactor
- [x] 10-02-PLAN.md — Stop/On/Off/Status CLI subcommands + dispatch handlers + enabled-check gating
- [x] 10-03-PLAN.md — Doctor diagnostic subcommand + plugin auto-start fix
- [x] 10-04-PLAN.md — Gap closure: add AI backend info to `aliast status` output

### Phase 11: CLI UX + NL Indicator
**Goal**: New users can get started from help output alone, and the NL mode indicator is visually clean
**Depends on**: Phase 10
**Requirements**: CLI-01, CLI-02, CLI-03, NL-01, NL-02
**Success Criteria** (what must be TRUE):
  1. Running `aliast -h` shows a compact subcommand overview that fits on one screen
  2. Running `aliast --help` includes AI setup guidance with environment variable names, Ollama instructions, and cloud backend configuration
  3. NL mode displays a colored unicode dot instead of `[NL]` text in the prompt
  4. CI workflow and Homebrew formula produce and reference the `aliast` binary name (not `aliast-daemon`)
**Plans**: 2 plans

Plans:
- [x] 11-01-PLAN.md — NL indicator: replace [NL] text with blue unicode dot via region_highlight
- [ ] 11-02-PLAN.md — Two-tier CLI help output with AI setup guidance + CLI-03 verification

## Progress

**Execution Order:**
Phases execute in numeric order: 9 -> 10 -> 11

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 1. Daemon + IPC + Ghost Text | v1.0 | 3/3 | Complete | 2026-04-02 |
| 2. History-Based Suggestions | v1.0 | 3/3 | Complete | 2026-04-03 |
| 3. Natural Language Mode | v1.0 | 3/3 | Complete | 2026-04-03 |
| 4. Context & Ranking Intelligence | v1.0 | 2/2 | Complete | 2026-04-03 |
| 5. Cloud Backends & Terminal Compat | v1.0 | 2/2 | Complete | 2026-04-03 |
| 6. Project Rename | v1.1 | 2/2 | Complete | 2026-04-03 |
| 7. CI/CD Release Pipeline | v1.1 | 1/1 | Complete | 2026-04-09 |
| 8. Homebrew Tap + Formula | v1.1 | 2/2 | Complete | 2026-04-10 |
| 9. Binary Rename + Version Foundations | v1.2 | 3/3 | Executing | - |
| 10. Daemon Lifecycle | v1.2 | 4/4 | Complete    | 2026-04-10 |
| 11. CLI UX + NL Indicator | v1.2 | 1/2 | Executing | - |
