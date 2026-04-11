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

## Usage

### Ghost Text

Start typing a command. Suggestions appear as dimmed text after your cursor. Press **Tab** to accept.

### Natural Language Mode

Press **Ctrl+Space** to toggle NL mode. A blue dot appears before your cursor. Type what you want in plain English, press **Enter**, and AliasT generates the shell command. Press **Escape** to exit NL mode.

### Commands

```
aliast start     Start the daemon
aliast stop      Stop the daemon
aliast status    Show daemon state, socket, and AI backend
aliast on        Enable suggestions (all shells)
aliast off       Disable suggestions (all shells)
aliast doctor    Run diagnostic health checks
```

## How It Works

A Rust daemon runs in the background, serving suggestions over a Unix socket. The zsh plugin connects to the daemon on first keystroke, renders ghost text via `POSTDISPLAY`, and handles the NL mode UI.

Suggestions are ranked by frecency -- frequency, recency, directory, and exit code all factor in. History stays local. Cloud API calls only happen in NL mode when you explicitly trigger them.

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `ALIAST_NL_MODEL` | AI model name | (none -- NL mode disabled) |
| `ALIAST_NL_BACKEND` | Backend: `ollama`, `claude`, `openai` | `ollama` |
| `ALIAST_ANTHROPIC_KEY` | API key for Claude | |
| `ALIAST_OPENAI_KEY` | API key for OpenAI | |
| `ALIAST_SUGGESTION_STYLE` | Ghost text style: `dark`, `light`, `solarized` | `dark` |
| `ALIAST_SUGGESTION_HIGHLIGHT` | Custom highlight spec (overrides style) | |
| `ALIAST_LOG_LEVEL` | Daemon log level | `warn` |

## Requirements

- macOS 13+ (Ventura or later)
- zsh (default on macOS)
- Homebrew

## License

[MIT](LICENSE)
