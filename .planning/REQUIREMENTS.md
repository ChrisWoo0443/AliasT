# Requirements: AliasT

**Defined:** 2026-04-10
**Core Value:** Type less, execute faster — ghost text suggestions appear as you type and a hotkey lets you describe what you want in plain English.

## v1.2 Requirements

Requirements for CLI polish & reliability milestone. Each maps to roadmap phases.

### Daemon Lifecycle

- [ ] **LIFE-01**: Daemon auto-starts on first plugin load (not just on reconnect)
- [x] **LIFE-02**: User can stop a running daemon via `aliast stop`
- [x] **LIFE-03**: User can toggle suggestions on/off via `aliast on` / `aliast off`
- [ ] **LIFE-04**: User can check system health via `aliast doctor` (daemon status, AI backend, plugin)

### Binary & Version

- [x] **BIN-01**: Binary is named `aliast` (not `aliast-daemon`)
- [ ] **BIN-02**: All workspace crates use `version.workspace = true` from root Cargo.toml
- [ ] **BIN-03**: Test assertions use `env!("CARGO_PKG_VERSION")` instead of hardcoded `"0.1.0"`

### CLI UX

- [ ] **CLI-01**: `aliast -h` shows compact subcommand overview
- [ ] **CLI-02**: `aliast --help` includes AI setup guidance (env vars, Ollama, cloud backends)
- [ ] **CLI-03**: CI workflow and Homebrew formula updated for `aliast` binary name

### NL Mode Indicator

- [ ] **NL-01**: NL mode displays unicode colored dot (`●`) instead of `[NL]` text
- [ ] **NL-02**: Indicator renders via PREDISPLAY + `region_highlight` with `P` flag for color

## Future Requirements

Deferred to future release. Tracked but not in current roadmap.

### Compatibility

- **COMPAT-01**: Compatibility symlink (`aliast-daemon` → `aliast`) for existing users

### Persistence

- **PERSIST-01**: On/off toggle state persists across daemon restarts

## Out of Scope

Explicitly excluded. Documented to prevent scope creep.

| Feature | Reason |
|---------|--------|
| Interactive setup wizard | Violates "config via dotfiles" constraint |
| PID file for daemon management | Race-prone; socket-based IPC is safer |
| ANSI escape codes in PREDISPLAY | Zsh doesn't support them there; use region_highlight |
| Non-macOS support | macOS-only constraint unchanged |
| Non-zsh shell support | zsh-only constraint unchanged |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| BIN-01 | Phase 9 | Complete |
| BIN-02 | Phase 9 | Pending |
| BIN-03 | Phase 9 | Pending |
| LIFE-01 | Phase 10 | Pending |
| LIFE-02 | Phase 10 | Complete |
| LIFE-03 | Phase 10 | Complete |
| LIFE-04 | Phase 10 | Pending |
| CLI-01 | Phase 11 | Pending |
| CLI-02 | Phase 11 | Pending |
| CLI-03 | Phase 11 | Pending |
| NL-01 | Phase 11 | Pending |
| NL-02 | Phase 11 | Pending |

**Coverage:**
- v1.2 requirements: 12 total
- Mapped to phases: 12
- Unmapped: 0

---
*Requirements defined: 2026-04-10*
*Last updated: 2026-04-10 after roadmap creation*
