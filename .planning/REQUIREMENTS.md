# Requirements: Alias

**Defined:** 2026-04-02
**Core Value:** Type less, execute faster -- ghost text suggestions appear as you type and a hotkey lets you describe what you want in plain English.

## v1 Requirements

Requirements for initial release. Each maps to roadmap phases.

### Core Infrastructure

- [x] **INFRA-01**: Rust daemon listens on a Unix domain socket and processes suggestion requests asynchronously
- [x] **INFRA-02**: Zsh ZLE widget renders ghost-text suggestions via POSTDISPLAY with dimmed styling
- [x] **INFRA-03**: Zsh plugin communicates with daemon non-blockingly via zle -F fd callbacks
- [x] **INFRA-04**: NDJSON protocol between zsh plugin and Rust daemon for structured message exchange

### Inline Suggestions

- [x] **SUGG-01**: User sees ghost-text suggestion from shell history within 100ms of typing
- [ ] **SUGG-02**: User can accept full suggestion with Tab or Right-arrow
- [ ] **SUGG-03**: User can accept suggestion word-by-word with Alt+Right
- [ ] **SUGG-04**: Suggestions are ranked by frecency (recency + frequency + directory affinity + exit code)
- [ ] **SUGG-05**: Suggestions are context-aware using cwd, git status, last exit code, and env vars
- [x] **SUGG-06**: History is indexed in SQLite with metadata (command, timestamp, cwd, exit code, duration)

### Natural Language Mode

- [ ] **NL-01**: User can toggle into natural language mode via a configurable hotkey
- [x] **NL-02**: User types plain English and receives a generated shell command
- [ ] **NL-03**: Generated command appears in editable buffer for review -- never auto-executes
- [ ] **NL-04**: User can accept (Enter), edit, or reject (Escape) generated commands
- [x] **NL-05**: Ollama backend works out of the box for local-first AI (no API keys required)
- [ ] **NL-06**: Cloud AI backends supported (Claude API, OpenAI API) via API key configuration
- [x] **NL-07**: AI backend is abstracted behind a trait so backends are swappable via config

### Terminal Compatibility

- [ ] **TERM-01**: Plugin works in iTerm2, Terminal.app, Kitty, Ghostty, and Alacritty
- [ ] **TERM-02**: Ghost-text styling is configurable to handle different terminal color schemes

## v2 Requirements

Deferred to future release. Tracked but not in current roadmap.

### Enhanced Intelligence

- **INTEL-01**: AI-powered inline completion (tiered: instant history, async AI replacement)
- **INTEL-02**: Command explanation on demand via hotkey
- **INTEL-03**: Error-aware next-command suggestion after non-zero exit

### Configuration & Distribution

- **DIST-01**: TOML configuration file at ~/.config/alias/config.toml
- **DIST-02**: Homebrew tap for daemon binary installation
- **DIST-03**: Plugin manager compatibility (oh-my-zsh, zinit, antidote)
- **DIST-04**: Snippet/abbreviation expansion from user-defined shortcuts

## Out of Scope

Explicitly excluded. Documented to prevent scope creep.

| Feature | Reason |
|---------|--------|
| Full terminal emulator | Massive scope -- Warp has 100+ engineers. Plugin composability beats monoliths. |
| Auto-execute AI commands | Catastrophic risk -- AI models hallucinate. Every serious tool requires confirmation. |
| Dropdown completion menu | Ghost text is simpler, less intrusive, fewer terminal compat issues. |
| Multi-shell support (bash/fish) | Each shell has different line editing APIs. Zsh-only for v1. |
| Cross-platform (Linux/Windows) | macOS + zsh is the tightest target. Rust compiles cross-platform for later. |
| Telemetry/analytics | Privacy is core differentiator. Zero telemetry, all data local. |
| Cloud history sync | Adds server infra and trust burden. Atuin already does this well. |
| Real-time streaming suggestions | Flickering ghost text is distracting. Show complete suggestion only. |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| INFRA-01 | Phase 1 | Complete |
| INFRA-02 | Phase 1 | Complete |
| INFRA-03 | Phase 1 | Complete |
| INFRA-04 | Phase 1 | Complete |
| SUGG-01 | Phase 2 | Complete |
| SUGG-02 | Phase 2 | Pending |
| SUGG-03 | Phase 2 | Pending |
| SUGG-04 | Phase 4 | Pending |
| SUGG-05 | Phase 4 | Pending |
| SUGG-06 | Phase 2 | Complete |
| NL-01 | Phase 3 | Pending |
| NL-02 | Phase 3 | Complete |
| NL-03 | Phase 3 | Pending |
| NL-04 | Phase 3 | Pending |
| NL-05 | Phase 3 | Complete |
| NL-06 | Phase 5 | Pending |
| NL-07 | Phase 3 | Complete |
| TERM-01 | Phase 5 | Pending |
| TERM-02 | Phase 5 | Pending |

**Coverage:**
- v1 requirements: 19 total
- Mapped to phases: 19
- Unmapped: 0

---
*Requirements defined: 2026-04-02*
*Last updated: 2026-04-02 after roadmap creation*
