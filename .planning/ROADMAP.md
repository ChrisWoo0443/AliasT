# Roadmap: Alias

## Overview

Alias ships as a vertical build: daemon plumbing and ghost-text rendering first, then history-based suggestions to validate the core typing loop, then natural language mode with local AI as the key differentiator, then context intelligence to make suggestions smarter, and finally cloud backends and terminal compatibility to broaden reach. Each phase delivers a testable, end-to-end capability. Phases 1-2 produce a usable product with zero AI dependency; Phase 3 is the feature that justifies Alias over zsh-autosuggestions.

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [ ] **Phase 1: Daemon + IPC + Ghost Text** - Rust daemon, zsh plugin, Unix socket IPC, and ghost-text rendering plumbing
- [ ] **Phase 2: History-Based Suggestions** - Shell history indexing and inline ghost-text completion with accept/reject keybindings
- [ ] **Phase 3: Natural Language Mode** - Hotkey-activated NL-to-command with local Ollama backend and pluggable AI trait
- [ ] **Phase 4: Context & Ranking Intelligence** - Context-aware suggestions and frecency-based ranking
- [ ] **Phase 5: Cloud Backends & Terminal Compatibility** - Cloud AI providers and multi-terminal support with configurable styling

## Phase Details

### Phase 1: Daemon + IPC + Ghost Text
**Goal**: A running Rust daemon and zsh plugin can communicate over a Unix socket and render ghost text in the terminal
**Depends on**: Nothing (first phase)
**Requirements**: INFRA-01, INFRA-02, INFRA-03, INFRA-04
**Success Criteria** (what must be TRUE):
  1. User sources the zsh plugin and the Rust daemon starts automatically in the background
  2. User types in the terminal and sees a dimmed ghost-text placeholder rendered after the cursor (hardcoded test string)
  3. Daemon and plugin exchange structured NDJSON messages over a Unix domain socket without blocking the shell
  4. Daemon cleans up stale socket files on startup and shuts down gracefully on signal
**Plans**: 3 plans

Plans:
- [x] 01-01-PLAN.md -- Cargo workspace scaffolding, NDJSON protocol types, core suggestion function, daemon CLI skeleton
- [x] 01-02-PLAN.md -- Daemon server implementation: socket listener, connection handler, lifecycle management, graceful shutdown
- [x] 01-03-PLAN.md -- Zsh plugin with IPC and ghost text rendering, end-to-end integration tests, visual verification

### Phase 2: History-Based Suggestions
**Goal**: Users see relevant command suggestions from their shell history as they type, and can accept them with familiar keybindings
**Depends on**: Phase 1
**Requirements**: SUGG-01, SUGG-02, SUGG-03, SUGG-06
**Success Criteria** (what must be TRUE):
  1. User types a partial command and sees a ghost-text suggestion from shell history within 100ms
  2. User presses Tab or Right-arrow and the full suggestion is accepted into the command buffer
  3. User presses Alt+Right and the suggestion is accepted one word at a time
  4. Shell history is imported and indexed in SQLite with metadata (command, timestamp, cwd, exit code, duration)
**Plans**: TBD

Plans:
- [ ] 02-01: TBD
- [ ] 02-02: TBD

### Phase 3: Natural Language Mode
**Goal**: Users can describe what they want in plain English and get a generated shell command for review, powered by a local LLM
**Depends on**: Phase 2
**Requirements**: NL-01, NL-02, NL-03, NL-04, NL-05, NL-07
**Success Criteria** (what must be TRUE):
  1. User presses a configurable hotkey and enters natural language mode with a visible mode indicator
  2. User types a plain English description and receives a generated shell command
  3. Generated command appears in the editable buffer -- never auto-executes
  4. User can accept (Enter), edit inline, or reject (Escape) the generated command
  5. Ollama backend works without any API key configuration for local-first AI
**Plans**: TBD

Plans:
- [ ] 03-01: TBD
- [ ] 03-02: TBD

### Phase 4: Context & Ranking Intelligence
**Goal**: Suggestions are smarter -- ranked by usage patterns and enriched with environmental context
**Depends on**: Phase 2
**Requirements**: SUGG-04, SUGG-05
**Success Criteria** (what must be TRUE):
  1. Suggestions are ranked by frecency (recency + frequency + directory affinity + exit code weighting)
  2. Suggestions reflect the user's current context -- different suggestions appear in different directories, git repos, and after different exit codes
**Plans**: TBD

Plans:
- [ ] 04-01: TBD

### Phase 5: Cloud Backends & Terminal Compatibility
**Goal**: Users can opt into cloud AI providers for higher-quality generation, and the plugin works reliably across major macOS terminals
**Depends on**: Phase 3
**Requirements**: NL-06, TERM-01, TERM-02
**Success Criteria** (what must be TRUE):
  1. User can configure Claude API or OpenAI API keys and use cloud backends for natural language command generation
  2. Ghost-text rendering works correctly in iTerm2, Terminal.app, Kitty, Ghostty, and Alacritty
  3. Ghost-text styling (color, dimming) is configurable to accommodate different terminal color schemes
**Plans**: TBD

Plans:
- [ ] 05-01: TBD
- [ ] 05-02: TBD

## Progress

**Execution Order:**
Phases execute in numeric order: 1 -> 2 -> 3 -> 4 -> 5

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Daemon + IPC + Ghost Text | 0/3 | Planning complete | - |
| 2. History-Based Suggestions | 0/0 | Not started | - |
| 3. Natural Language Mode | 0/0 | Not started | - |
| 4. Context & Ranking Intelligence | 0/0 | Not started | - |
| 5. Cloud Backends & Terminal Compatibility | 0/0 | Not started | - |
