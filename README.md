# AliasT

Ghost-text autocompletion and natural-language commands for zsh. Works in any terminal emulator -- iTerm, Terminal.app, Kitty, etc.

Type less, execute faster. Suggestions appear inline as you type. A hotkey lets you describe what you want in plain English and get the command back.

## Install

```bash
brew tap ChrisWoo0443/aliast
brew install aliast
```

Add to your `~/.zshrc`:

```bash
source $(brew --prefix)/share/aliast/aliast.plugin.zsh
```

Restart your terminal. The daemon starts automatically.

## AI Setup

AliasT uses a local or cloud AI backend for natural-language mode.

### Ollama (free, local)

```bash
brew install ollama
ollama serve
ollama pull llama3.2
```

Add to `~/.zshrc` (before the plugin source line):

```bash
export ALIAST_NL_MODEL=llama3.2
```

### Claude

```bash
export ALIAST_NL_BACKEND=claude
export ALIAST_NL_MODEL=claude-sonnet-4-20250514
export ALIAST_ANTHROPIC_KEY=sk-ant-...
```

### OpenAI

```bash
export ALIAST_NL_BACKEND=openai
export ALIAST_NL_MODEL=gpt-4o
export ALIAST_OPENAI_KEY=sk-...
```

Run `aliast doctor` to verify your setup.

> **Note:** The daemon reads these variables once at startup. After changing any `ALIAST_NL_*` variable, run `aliast restart` for it to take effect.

## Usage

### Ghost Text

Start typing a command. Suggestions appear as dimmed text after your cursor. Press **Tab** to accept the whole suggestion, or **Shift+Tab** to accept just the next word. When the top match isn't the one you want, **Ctrl+N** / **Ctrl+P** cycle through the other candidates (with no suggestion showing, they do normal history navigation).

Suggestions come from three sources, in priority order: your own history
(frecency-ranked), a bundled grammar pack for common CLI tools (so `git sw`
completes to `switch` even if you've never typed it -- git, docker, cargo,
brew, npm, kubectl, gh, and more), and real directory completion for
`cd`/`ls`/`pushd`/`mkdir` (subdirectories of where you are, ranked by where
you actually go).

### Natural Language Mode

Press **Ctrl+Space** to toggle NL mode. A blue dot appears before your cursor. Type what you want in plain English, press **Enter**, and AliasT generates the shell command. Press **Escape** to exit NL mode.

Generated commands that look destructive (`rm -rf`, `sudo`, `curl | sh`, writes to raw devices, ...) are tinted red in the review buffer so you look twice before pressing Enter.

Some terminals do not emit Ctrl+Space. If the toggle does nothing, rebind it with `export ALIAST_NL_KEY='^G'` (bindkey syntax) before sourcing the plugin.

### Commands

```
aliast start     Start the daemon (also re-enables auto-start)
aliast stop      Stop the daemon and pause auto-start until `aliast start`
aliast restart   Restart the daemon (picks up changed ALIAST_* env vars)
aliast status    Show daemon state, socket, and AI backend
aliast on        Enable suggestions (all shells)
aliast off       Disable suggestions (all shells)
aliast doctor    Run diagnostic health checks
aliast logs      Show the daemon log (last 50 lines)
aliast import    Import new entries from ~/.zsh_history (dedup-safe)
aliast stats     Show top commands and most-accepted suggestions
```

The plugin normally starts the daemon on demand, so `aliast stop` also pauses that auto-start -- otherwise the next keystroke would just respawn it. To pause suggestions but keep the daemon running, use `aliast off`.

## How It Works

A Rust daemon runs in the background, serving suggestions over a Unix socket. The zsh plugin connects to the daemon on first keystroke, renders ghost text via `POSTDISPLAY`, and handles the NL mode UI.

Suggestions are ranked by frecency -- frequency, recency, directory, and exit
code all factor in. When history has no answer, AliasT falls back to a bundled
CLI grammar pack and to real subdirectories of your current directory. History stays local. Cloud API calls only happen in NL mode when you explicitly trigger them -- and only your prompt plus a small context block (current directory, git branch, and last exit code) is sent. Set `ALIAST_NL_NO_CONTEXT=1` to send only the prompt.

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `ALIAST_NL_MODEL` | AI model name | (none -- NL mode disabled) |
| `ALIAST_NL_BACKEND` | Backend: `ollama`, `claude`, `openai` | `ollama` |
| `ALIAST_ANTHROPIC_KEY` | API key for Claude | |
| `ALIAST_OPENAI_KEY` | API key for OpenAI | |
| `ALIAST_NL_NO_CONTEXT` | Send only the prompt to the AI (omit cwd/branch/exit code) | (unset) |
| `ALIAST_NL_KEY` | Key that toggles NL mode (bindkey syntax) | `^ ` (Ctrl+Space) |
| `ALIAST_SUGGESTION_STYLE` | Ghost text style: `dark`, `light`, `solarized` | `dark` |
| `ALIAST_SUGGESTION_HIGHLIGHT` | Custom highlight spec (overrides style) | |
| `ALIAST_LOG_LEVEL` | Daemon log level | `warn` |

## Requirements

- macOS 13+ (Ventura or later)
- zsh (default on macOS)
- Homebrew

## License

[MIT](LICENSE)
