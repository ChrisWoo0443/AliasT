# alias.plugin.zsh -- Ghost text suggestions via a Rust daemon
# Connects to alias-daemon over a Unix socket, sends NDJSON requests on
# keystrokes, and renders ghost text after the cursor with POSTDISPLAY.

# ── 1. Guard and module loading ──────────────────────────────────────
(( ${+_ALIAS_LOADED} )) && return
typeset -g _ALIAS_LOADED=1

zmodload zsh/net/socket || {
  print "alias: zsh/net/socket module required" >&2
  return 1
}
autoload -Uz add-zle-hook-widget

# ── 2. State variables ──────────────────────────────────────────────
typeset -g _ALIAS_FD=""
typeset -g _ALIAS_REQ_ID=0
typeset -g _ALIAS_HIGHLIGHT_ENTRY=""
typeset -g _ALIAS_LAST_BUFFER=""
typeset -g _ALIAS_SOCKET_PATH="${XDG_RUNTIME_DIR:-/tmp/alias-$UID}/alias/alias.sock"

# ── 3. Connection management (lazy -- no I/O at plugin load time) ───
_alias_connect() {
  # Already connected
  [[ -n "$_ALIAS_FD" ]] && return 0

  zsocket "$_ALIAS_SOCKET_PATH" 2>/dev/null || return 1
  _ALIAS_FD=$REPLY
  return 0
}

_alias_disconnect() {
  [[ -z "$_ALIAS_FD" ]] && return
  exec {_ALIAS_FD}>&-
  _ALIAS_FD=""
}

_alias_reconnect() {
  _alias_disconnect
  # Attempt daemon respawn (fire-and-forget)
  command alias-daemon start &>/dev/null &!
}

# ── 4. Ghost text rendering ─────────────────────────────────────────
_alias_show_ghost() {
  local suggestion_text="$1"

  _alias_clear_ghost

  POSTDISPLAY="$suggestion_text"

  if (( $#POSTDISPLAY > 0 )); then
    local start=$#BUFFER
    local end=$(( start + $#POSTDISPLAY ))
    local style="${ALIAS_SUGGESTION_HIGHLIGHT:-fg=8}"
    _ALIAS_HIGHLIGHT_ENTRY="${start} ${end} ${style} memo=alias"
    region_highlight+=("$_ALIAS_HIGHLIGHT_ENTRY")
  fi
}

_alias_clear_ghost() {
  if [[ -n "$_ALIAS_HIGHLIGHT_ENTRY" ]]; then
    region_highlight=("${(@)region_highlight:#$_ALIAS_HIGHLIGHT_ENTRY}")
    _ALIAS_HIGHLIGHT_ENTRY=""
  fi
  POSTDISPLAY=""
}

# ── 5. IPC -- send request and read response synchronously ──────────
_alias_request_suggestion() {
  _alias_connect || return

  (( _ALIAS_REQ_ID++ ))

  # Escape buffer for JSON
  local escaped_buffer="${BUFFER//\\/\\\\}"
  escaped_buffer="${escaped_buffer//\"/\\\"}"

  local msg="{\"id\":\"r${_ALIAS_REQ_ID}\",\"type\":\"complete\",\"buf\":\"${escaped_buffer}\",\"cur\":${CURSOR}}"

  print -u $_ALIAS_FD "$msg" 2>/dev/null || {
    _alias_reconnect
    return
  }

  # Read response synchronously (with short timeout to avoid blocking)
  local line=""
  if read -r -u $_ALIAS_FD -t 0.1 line; then
    if [[ "$line" == *'"type":"suggestion"'* ]]; then
      local text="${line##*\"text\":\"}"
      text="${text%%\"*}"

      if [[ -n "$text" ]]; then
        _alias_show_ghost "$text"
      else
        _alias_clear_ghost
      fi
    fi
  fi
}

# ── 6. Widget wrappers (minimal) ────────────────────────────────────
# Use .self-insert / .accept-line (dot-prefixed builtins) to avoid
# infinite recursion when other plugins have already wrapped self-insert.
_alias_self_insert_wrapper() {
  zle .self-insert "$@"
  _alias_request_suggestion
}
zle -N self-insert _alias_self_insert_wrapper

_alias_accept_line_wrapper() {
  _alias_clear_ghost
  zle .accept-line "$@"
}
zle -N accept-line _alias_accept_line_wrapper

# ── 7. Hook registration (non-conflicting) ──────────────────────────
_alias_line_pre_redraw() {
  if [[ "$BUFFER" != "$_ALIAS_LAST_BUFFER" ]]; then
    _ALIAS_LAST_BUFFER="$BUFFER"
    if [[ -z "$BUFFER" ]]; then
      _alias_clear_ghost
    fi
  fi
}
add-zle-hook-widget zle-line-pre-redraw _alias_line_pre_redraw

# ── 8. Compatibility detection ──────────────────────────────────────
if (( ${+_ZSH_AUTOSUGGEST_HIGHLIGHT_STYLE} )) || [[ -n "${functions[_zsh_autosuggest_suggest]}" ]]; then
  print "alias: zsh-autosuggestions detected. Ghost text may conflict." >&2
fi
