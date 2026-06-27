# Terminal Compatibility

Ghost text rendering compatibility across macOS terminals.

## Test Matrix

"Verified" below means the item was manually checked; "Not tested" is unverified
(no known problem, just not exercised yet). So far only **ghost-text visibility**
has been verified -- style presets and the Ctrl+Space NL toggle are still unverified.

| Terminal      | Ghost Text Visible | Style Presets | NL Toggle (Ctrl+Space) |
|---------------|--------------------|---------------|------------------------|
| Terminal.app  | Verified           | Not tested    | Not tested             |
| iTerm2        | Verified           | Not tested    | Not tested             |
| Kitty         | Verified           | Not tested    | Not tested             |
| Ghostty       | Not tested         | Not tested    | Not tested             |
| Alacritty     | Not tested         | Not tested    | Not tested             |

> **Ctrl+Space caveat:** NL mode is bound to Ctrl+Space (NUL), which some terminals
> do not emit. If it does nothing in your terminal, rebind it in `~/.zshrc` after
> sourcing the plugin, e.g. `bindkey '^G' _aliast_nl_toggle`.

## Installation

```bash
# Terminal.app -- built-in, no install needed

# iTerm2
brew install --cask iterm2

# Kitty
brew install --cask kitty

# Ghostty
# Download from https://ghostty.org

# Alacritty
brew install --cask alacritty
```

## Style Preset Test Instructions

For each terminal:

1. Open the terminal
2. Source the plugin:
   ```bash
   source plugin/aliast.plugin.zsh
   ```
3. Ensure daemon is running:
   ```bash
   aliast start &
   ```
4. Type a partial command that has history matches
5. Verify ghost text appears in dimmed gray after the cursor

### Testing Each Preset

```bash
# Dark preset (default) -- medium gray on dark backgrounds
ALIAST_SUGGESTION_STYLE=dark source plugin/aliast.plugin.zsh

# Light preset -- darker gray on light backgrounds
ALIAST_SUGGESTION_STYLE=light source plugin/aliast.plugin.zsh

# Solarized preset -- Solarized base01, visible on both dark and light
ALIAST_SUGGESTION_STYLE=solarized source plugin/aliast.plugin.zsh

# Custom override -- verifies ALIAST_SUGGESTION_HIGHLIGHT takes priority
ALIAST_SUGGESTION_HIGHLIGHT="fg=red" source plugin/aliast.plugin.zsh
```

### What to Look For

- Ghost text is visible (not invisible or same color as background)
- Colors look appropriate for the theme (not too bright, not invisible)
- POSTDISPLAY renders inline after cursor (not on a new line)
- Tab acceptance works (ghost text becomes real buffer text)
- No rendering artifacts or flickering

## Test Results

Ghost-text rendering verified in Terminal.app, iTerm2, and Kitty. Style presets
and the Ctrl+Space NL toggle have not yet been verified in any terminal.

## Known Quirks

- Ctrl+Space (NUL) is not emitted by every terminal; see the caveat above.
