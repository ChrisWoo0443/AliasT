# Roadmap: AliasT

## Milestones

- ✅ **v1.0 MVP** - Phases 1-5 (shipped 2026-04-03)
- 🚧 **v1.1 AliasT + Homebrew Distribution** - Phases 6-8 (in progress)

## Phases

<details>
<summary>✅ v1.0 MVP (Phases 1-5) - SHIPPED 2026-04-03</summary>

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

- [x] **Phase 1: Daemon + IPC + Ghost Text** - Rust daemon, zsh plugin, Unix socket IPC, and ghost-text rendering plumbing
- [x] **Phase 2: History-Based Suggestions** - Shell history indexing and inline ghost-text completion with accept/reject keybindings
- [x] **Phase 3: Natural Language Mode** - Hotkey-activated NL-to-command with local Ollama backend and pluggable AI trait
- [x] **Phase 4: Context & Ranking Intelligence** - Context-aware suggestions and frecency-based ranking
- [x] **Phase 5: Cloud Backends & Terminal Compatibility** - Cloud AI providers and multi-terminal support with configurable styling

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
  2. User presses Tab and the full suggestion is accepted into the command buffer
  3. User presses Shift+Tab and the suggestion is accepted one word at a time
  4. Shell history is imported and indexed in SQLite with metadata (command, timestamp, cwd)
**Plans**: 3 plans

Plans:
- [x] 02-01-PLAN.md -- HistoryStore with SQLite prefix search, zsh history parser, Record/Ack protocol messages, rewired suggest()
- [x] 02-02-PLAN.md -- Daemon integration: SQLite init on startup, history import, connection handler wiring for Record and Complete
- [x] 02-03-PLAN.md -- Zsh plugin Tab/Shift+Tab keybindings, precmd command recording hook, interactive verification

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
**Plans**: 3 plans

Plans:
- [x] 03-01-PLAN.md -- Protocol types (Generate/Command), AiBackend trait, OllamaBackend with reqwest
- [x] 03-02-PLAN.md -- Daemon wiring: AI backend init, connection handler Generate dispatch, E2E tests
- [x] 03-03-PLAN.md -- Zsh plugin NL mode: Ctrl+Space toggle, spinner, accept/reject keybindings

### Phase 4: Context & Ranking Intelligence
**Goal**: Suggestions are smarter -- ranked by usage patterns and enriched with environmental context
**Depends on**: Phase 2
**Requirements**: SUGG-04, SUGG-05
**Success Criteria** (what must be TRUE):
  1. Suggestions are ranked by frecency (recency + frequency + directory affinity + exit code weighting)
  2. Suggestions reflect the user's current context -- different suggestions appear in different directories, git repos, and after different exit codes
**Plans**: 2 plans

Plans:
- [x] 04-01-PLAN.md -- Protocol extension, schema migration, frecency SQL query, suggest() context-aware, daemon dispatch, AI prompt enrichment
- [x] 04-02-PLAN.md -- Zsh plugin exit code capture, context gathering for Complete/Record/Generate, git branch caching

### Phase 5: Cloud Backends & Terminal Compatibility
**Goal**: Users can opt into cloud AI providers for higher-quality generation, and the plugin works reliably across major macOS terminals
**Depends on**: Phase 3
**Requirements**: NL-06, TERM-01, TERM-02
**Success Criteria** (what must be TRUE):
  1. User can configure Claude API or OpenAI API keys and use cloud backends for natural language command generation
  2. Ghost-text rendering works correctly in iTerm2, Terminal.app, Kitty, Ghostty, and Alacritty
  3. Ghost-text styling (color, dimming) is configurable to accommodate different terminal color schemes
**Plans**: 2 plans

Plans:
- [x] 05-01-PLAN.md -- ClaudeBackend and OpenAiBackend implementing AiBackend trait, daemon backend selection via ALIAS_NL_BACKEND
- [x] 05-02-PLAN.md -- Style presets (dark/light/solarized) for ghost text, terminal compatibility verification

</details>

### 🚧 v1.1 AliasT + Homebrew Distribution (In Progress)

**Milestone Goal:** Rename the project from Alias to AliasT across all code/configs/binaries, then make it installable via `brew tap + brew install`.

- [ ] **Phase 6: Project Rename** - Full rename from Alias to AliasT across all crate names, binaries, env vars, paths, and plugin files
- [ ] **Phase 7: CI/CD Release Pipeline** - GitHub Actions workflow that builds arch-specific binaries and creates GitHub Releases on version tags
- [ ] **Phase 8: Homebrew Tap + Formula** - Custom tap repository with formula delivering daemon binary and plugin via `brew install`

## Phase Details

### Phase 6: Project Rename
**Goal**: Every user-visible and internal identifier uses `aliast` instead of `alias`, and existing users retain their shell history
**Depends on**: Phase 5 (v1.0 complete)
**Requirements**: REN-01, REN-02, REN-03, REN-04, REN-05, REN-06, REN-07
**Success Criteria** (what must be TRUE):
  1. `cargo build` produces a binary named `aliast-daemon` and all three crates are named aliast-protocol, aliast-core, aliast-daemon
  2. User sources `aliast.plugin.zsh` and the plugin connects to the daemon using `ALIAST_*` env vars and `aliast/` socket paths
  3. User with an existing `~/.local/share/alias/history.db` starts the daemon and their history is automatically available at the new `~/.local/share/aliast/history.db` path
  4. All 106+ existing tests pass with the renamed identifiers
  5. `grep -r 'alias[-_]daemon\|ALIAS_\|_alias_\|alias\.plugin' --include='*.rs' --include='*.zsh' --include='*.toml'` returns zero hits for old naming (only `aliast` variants)
**Plans**: 2 plans

Plans:
- [ ] 06-01-PLAN.md -- Rename Rust crates (protocol, core, daemon), update all imports/env vars/paths, add data migration
- [x] 06-02-PLAN.md -- Rename zsh plugin file, replace all identifiers, update docs, rename GitHub repo

### Phase 7: CI/CD Release Pipeline
**Goal**: Pushing a version tag produces a GitHub Release with downloadable macOS binaries for both architectures
**Depends on**: Phase 6
**Requirements**: CI-01, CI-02, CI-03
**Success Criteria** (what must be TRUE):
  1. Pushing a `v*.*.*` tag to GitHub triggers the release workflow automatically
  2. The workflow produces both `aliast-daemon-aarch64-apple-darwin.tar.gz` and `aliast-daemon-x86_64-apple-darwin.tar.gz` artifacts
  3. A GitHub Release is created with the tag name, containing the tarballed binaries, the plugin file, and SHA256 checksums
**Plans**: 1 plan

Plans:
- [ ] 07-01-PLAN.md -- Release workflow: tag-triggered matrix build on native macOS runners, tarball packaging, GitHub Release creation

### Phase 8: Homebrew Tap + Formula
**Goal**: Users can install AliasT with `brew tap cwoo017/aliast && brew install aliast` and have a working daemon + plugin
**Depends on**: Phase 7
**Requirements**: BREW-01, BREW-02, BREW-03, BREW-04
**Success Criteria** (what must be TRUE):
  1. `brew tap cwoo017/aliast && brew install aliast` succeeds on a clean machine
  2. After install, `aliast-daemon --version` works (binary is on PATH) and the plugin file exists at `$HOMEBREW_PREFIX/share/aliast/aliast.plugin.zsh`
  3. `brew install` prints caveats with the exact `source` line the user needs to add to `.zshrc`
  4. Formula works on both Apple Silicon (`arm64`) and Intel (`x86_64`) Macs, downloading the correct architecture-specific binary
**Plans**: TBD

## Progress

**Execution Order:**
Phases execute in numeric order: 6 -> 7 -> 8

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 1. Daemon + IPC + Ghost Text | v1.0 | 3/3 | Complete | 2026-04-02 |
| 2. History-Based Suggestions | v1.0 | 3/3 | Complete | 2026-04-03 |
| 3. Natural Language Mode | v1.0 | 3/3 | Complete | 2026-04-03 |
| 4. Context & Ranking Intelligence | v1.0 | 2/2 | Complete | 2026-04-03 |
| 5. Cloud Backends & Terminal Compat | v1.0 | 2/2 | Complete | 2026-04-03 |
| 6. Project Rename | v1.1 | 0/2 | Planned | - |
| 7. CI/CD Release Pipeline | v1.1 | 0/1 | Planned | - |
| 8. Homebrew Tap + Formula | v1.1 | 0/0 | Not started | - |
