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
typeset -g _ALIAS_LAST_HISTNUM=0
typeset -g _ALIAS_SOCKET_PATH="${XDG_RUNTIME_DIR:-/tmp/alias-$UID}/alias/alias.sock"
typeset -g _ALIAS_NL_STATE="inactive"   # inactive | input | generating | review
typeset -g _ALIAS_NL_PROMPT=""           # Saved prompt for regeneration
typeset -g _ALIAS_LAST_EXIT=""
typeset -g _ALIAS_GIT_BRANCH=""

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
  # Remove all alias-owned highlight entries (memo=alias)
  region_highlight=("${(@)region_highlight:#* memo=alias}")
  _ALIAS_HIGHLIGHT_ENTRY=""
  POSTDISPLAY=""
}

# ── 5. IPC -- send request and read response synchronously ──────────
_alias_request_suggestion() {
  _alias_connect || return

  (( _ALIAS_REQ_ID++ ))

  # Escape buffer for JSON
  local escaped_buffer="${BUFFER//\\/\\\\}"
  escaped_buffer="${escaped_buffer//\"/\\\"}"

  local escaped_cwd="${PWD//\\/\\\\}"
  escaped_cwd="${escaped_cwd//\"/\\\"}"

  # Git branch: use cached value from precmd (per D-05)
  local branch_field=""
  if [[ -n "$_ALIAS_GIT_BRANCH" ]]; then
    local escaped_branch="${_ALIAS_GIT_BRANCH//\\/\\\\}"
    escaped_branch="${escaped_branch//\"/\\\"}"
    branch_field=",\"git_branch\":\"${escaped_branch}\""
  fi

  local exit_field=""
  if [[ -n "$_ALIAS_LAST_EXIT" ]]; then
    exit_field=",\"exit_code\":${_ALIAS_LAST_EXIT}"
  fi

  local msg="{\"id\":\"r${_ALIAS_REQ_ID}\",\"type\":\"complete\",\"buf\":\"${escaped_buffer}\",\"cur\":${CURSOR},\"cwd\":\"${escaped_cwd}\"${exit_field}${branch_field}}"

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
  # Skip ghost text suggestions during NL mode (PREDISPLAY conflicts)
  [[ "$_ALIAS_NL_STATE" != "inactive" ]] && return
  _alias_request_suggestion
}
zle -N self-insert _alias_self_insert_wrapper

_alias_nl_aware_accept() {
  case "$_ALIAS_NL_STATE" in
    input)
      # Enter in NL input mode → generate command
      if [[ -z "$BUFFER" ]]; then
        return  # Empty prompt, do nothing
      fi
      _ALIAS_NL_STATE="generating"
      _alias_nl_generate
      ;;
    review)
      # Enter on generated command → execute it, then stay in NL mode
      PREDISPLAY=""
      zle .accept-line
      # precmd will fire, then we re-enter NL input mode
      ;;
    *)
      # Normal mode — clear ghost, accept line
      _alias_clear_ghost
      zle .accept-line
      ;;
  esac
}
zle -N accept-line _alias_nl_aware_accept

# ── 7. Accept keybindings ───────────────────────────────────────────
_alias_accept_suggestion() {
  if [[ -n "$POSTDISPLAY" ]]; then
    # Save ghost text, clear display, then update buffer
    local ghost="$POSTDISPLAY"
    _alias_clear_ghost
    BUFFER="${BUFFER}${ghost}"
    CURSOR=$#BUFFER
  else
    zle expand-or-complete
  fi
}
zle -N _alias_accept_suggestion
bindkey '^I' _alias_accept_suggestion

_alias_accept_word() {
  if [[ -n "$POSTDISPLAY" ]]; then
    local remaining="$POSTDISPLAY"
    local word=""
    # Grab leading whitespace + next word
    if [[ "$remaining" =~ '^[[:space:]]*[^[:space:]]+' ]]; then
      word="${MATCH}"
    fi
    _alias_clear_ghost
    BUFFER="${BUFFER}${word}"
    CURSOR=$#BUFFER
    # Re-request suggestion for updated buffer
    _alias_request_suggestion
  else
    zle .complete-word
  fi
}
zle -N _alias_accept_word
bindkey '^[[Z' _alias_accept_word
bindkey '\e[Z' _alias_accept_word

# ── 8. Hook registration (non-conflicting) ──────────────────────────
_alias_line_pre_redraw() {
  if [[ "$BUFFER" != "$_ALIAS_LAST_BUFFER" ]]; then
    _ALIAS_LAST_BUFFER="$BUFFER"
    if [[ -z "$BUFFER" ]]; then
      _alias_clear_ghost
    fi
  fi
}
add-zle-hook-widget zle-line-pre-redraw _alias_line_pre_redraw

# ── 9. NL Mode: Natural Language to Command ────────────────────────

_alias_nl_deactivate() {
  _ALIAS_NL_STATE="inactive"
  _ALIAS_NL_PROMPT=""
  PREDISPLAY=""
  BUFFER=""
  CURSOR=0
  zle -R
}

_alias_nl_generate() {
  local prompt="$BUFFER"
  [[ -z "$prompt" && -n "$_ALIAS_NL_PROMPT" ]] && prompt="$_ALIAS_NL_PROMPT"
  [[ -z "$prompt" ]] && { _alias_nl_deactivate; return; }

  _ALIAS_NL_PROMPT="$prompt"
  BUFFER=""

  local tmpfile
  tmpfile=$(mktemp "${TMPDIR:-/tmp}/alias-nl.XXXXXX")

  # Disable job control to suppress [1] pid / [1] + done noise
  setopt local_options no_monitor

  # Background: open NEW connection, send generate request, write response
  {
    zmodload zsh/net/socket 2>/dev/null
    local bg_fd
    zsocket "$_ALIAS_SOCKET_PATH" 2>/dev/null
    bg_fd=$REPLY
    if [[ -z "$bg_fd" ]]; then
      echo "error:connect" > "$tmpfile"
      exit 1
    fi

    (( _ALIAS_REQ_ID++ ))
    local escaped="${prompt//\\/\\\\}"
    escaped="${escaped//\"/\\\"}"
    escaped="${escaped//$'\n'/ }"

    local escaped_cwd="${PWD//\\/\\\\}"
    escaped_cwd="${escaped_cwd//\"/\\\"}"
    local branch_field=""
    if [[ -n "$_ALIAS_GIT_BRANCH" ]]; then
      local escaped_branch="${_ALIAS_GIT_BRANCH//\\/\\\\}"
      escaped_branch="${escaped_branch//\"/\\\"}"
      branch_field=",\"git_branch\":\"${escaped_branch}\""
    fi
    local exit_field=""
    if [[ -n "$_ALIAS_LAST_EXIT" ]]; then
      exit_field=",\"exit_code\":${_ALIAS_LAST_EXIT}"
    fi

    local msg="{\"id\":\"r${_ALIAS_REQ_ID}\",\"type\":\"generate\",\"prompt\":\"${escaped}\",\"cwd\":\"${escaped_cwd}\"${exit_field}${branch_field}}"
    print -u $bg_fd "$msg" 2>/dev/null || { echo "error:send" > "$tmpfile"; exec {bg_fd}>&-; exit 1; }

    local line=""
    read -r -u $bg_fd -t 30 line 2>/dev/null || { echo "error:timeout" > "$tmpfile"; exec {bg_fd}>&-; exit 1; }
    echo "$line" > "$tmpfile"
    exec {bg_fd}>&-
  } &
  local bg_pid=$!

  # Foreground: spinner — clear buffer and show spinner in PREDISPLAY
  BUFFER=""
  CURSOR=0
  local spinner_chars='/-\|'
  local frame=0

  while kill -0 $bg_pid 2>/dev/null; do
    PREDISPLAY="[${spinner_chars:$((frame % 4)):1}] "
    BUFFER=""
    zle -R
    read -t 0.1 -k 1 key < /dev/tty 2>/dev/null
    if [[ "$key" == $'\e' ]]; then
      kill $bg_pid 2>/dev/null
      wait $bg_pid 2>/dev/null
      rm -f "$tmpfile"
      # Back to NL input
      _ALIAS_NL_STATE="input"
      PREDISPLAY="[NL] "
      BUFFER=""
      CURSOR=0
      zle -R
      return
    fi
    ((frame++))
  done
  wait $bg_pid 2>/dev/null

  # Read result from tmpfile
  local result
  result=$(<"$tmpfile")
  rm -f "$tmpfile"

  # Handle error responses
  if [[ "$result" == error:* ]]; then
    local error_type="${result#error:}"
    PREDISPLAY="[NL] "
    case "$error_type" in
      connect)
        BUFFER="# Error: cannot connect to daemon"
        ;;
      send|timeout)
        BUFFER="# Error: daemon communication failed"
        ;;
    esac
    _ALIAS_NL_STATE="review"
    CURSOR=$#BUFFER
    zle -R
    return
  fi

  # Parse JSON response -- extract "text" field for command, "msg" field for error
  if [[ "$result" == *'"type":"command"'* ]]; then
    local command_text="${result##*\"text\":\"}"
    command_text="${command_text%%\"*}"
    # Unescape basic JSON escapes
    command_text="${command_text//\\n/$'\n'}"
    command_text="${command_text//\\\\/\\}"
    command_text="${command_text//\\\"/\"}"
    PREDISPLAY="[NL] "
    BUFFER="$command_text"
    CURSOR=$#BUFFER
    _ALIAS_NL_STATE="review"
    zle -R
  elif [[ "$result" == *'"type":"error"'* ]]; then
    local error_msg="${result##*\"msg\":\"}"
    error_msg="${error_msg%%\"*}"
    PREDISPLAY="[NL] "
    BUFFER="# ${error_msg}"
    CURSOR=$#BUFFER
    _ALIAS_NL_STATE="review"
    zle -R
  else
    PREDISPLAY="[NL] "
    BUFFER="# Error: unexpected response"
    _ALIAS_NL_STATE="review"
    CURSOR=$#BUFFER
    zle -R
  fi
}

_alias_nl_toggle() {
  if [[ "$_ALIAS_NL_STATE" == "inactive" ]]; then
    # Toggle ON — enter NL mode
    _ALIAS_NL_STATE="input"
    _alias_clear_ghost
    PREDISPLAY="[NL] "
    BUFFER=""
    CURSOR=0
    zle -R
  else
    # Toggle OFF — exit NL mode entirely
    _alias_nl_deactivate
  fi
}
zle -N _alias_nl_toggle
bindkey '^ ' _alias_nl_toggle

_alias_nl_escape() {
  if [[ "$_ALIAS_NL_STATE" == "review" ]]; then
    # Escape in review → back to NL input (clear generated command)
    _ALIAS_NL_STATE="input"
    BUFFER=""
    CURSOR=0
    PREDISPLAY="[NL] "
    zle -R
  elif [[ "$_ALIAS_NL_STATE" != "inactive" ]]; then
    # Escape in input → exit NL mode
    _alias_nl_deactivate
  else
    zle send-break
  fi
}
zle -N _alias_nl_escape
bindkey '\e' _alias_nl_escape

# ── 10. Command recording (precmd hook -- registered FIRST) ────────
_alias_precmd_record() {
  # CRITICAL: Capture exit code FIRST -- $? is overwritten by every command
  local last_exit_code=$?
  # Store for use by _alias_request_suggestion and _alias_nl_generate
  typeset -g _ALIAS_LAST_EXIT=$last_exit_code

  # Cache git branch per prompt (not per keystroke) per D-05 and Pitfall 4
  typeset -g _ALIAS_GIT_BRANCH=""
  _ALIAS_GIT_BRANCH="$(git rev-parse --abbrev-ref HEAD 2>/dev/null)"

  # Get the most recent history entry number and command via fc
  local fc_out
  fc_out="$(fc -ln -1 2>/dev/null)" || return
  # fc -ln -1 outputs just the command (no number), with leading whitespace
  local cmd="${fc_out#"${fc_out%%[! ]*}"}"
  [[ -z "$cmd" ]] && return

  # Deduplicate using the command text itself
  [[ "$cmd" == "$_ALIAS_LAST_RECORDED" ]] && return
  typeset -g _ALIAS_LAST_RECORDED="$cmd"

  _alias_connect || return

  # Escape for JSON
  local escaped_cmd="${cmd//\\/\\\\}"
  escaped_cmd="${escaped_cmd//\"/\\\"}"
  local escaped_cwd="${PWD//\\/\\\\}"
  escaped_cwd="${escaped_cwd//\"/\\\"}"

  (( _ALIAS_REQ_ID++ ))
  local msg="{\"id\":\"r${_ALIAS_REQ_ID}\",\"type\":\"record\",\"cmd\":\"${escaped_cmd}\",\"cwd\":\"${escaped_cwd}\",\"exit_code\":${last_exit_code}}"

  # Send and read the Ack response to keep the socket buffer clean
  # (stale Ack would confuse the next suggestion read)
  print -u $_ALIAS_FD "$msg" 2>/dev/null || { _alias_reconnect; return }
  local ack=""
  read -r -u $_ALIAS_FD -t 0.2 ack 2>/dev/null
}

autoload -Uz add-zsh-hook
add-zsh-hook precmd _alias_precmd_record

# ── 11. NL mode persistence (re-enter after command execution) ─────
_alias_nl_precmd() {
  # If we just executed a command from NL review state, re-enter NL input
  if [[ "$_ALIAS_NL_STATE" == "review" ]]; then
    _ALIAS_NL_STATE="input"
    _ALIAS_NL_PROMPT=""
    # PREDISPLAY will be set when ZLE starts for the next prompt
  fi
}
add-zsh-hook precmd _alias_nl_precmd

# Re-apply [NL] prefix when ZLE starts a new line (after precmd)
_alias_nl_line_init() {
  if [[ "$_ALIAS_NL_STATE" == "input" ]]; then
    PREDISPLAY="[NL] "
  fi
}
add-zle-hook-widget zle-line-init _alias_nl_line_init

# ── 12. Compatibility detection ─────────────────────────────────────
if (( ${+_ZSH_AUTOSUGGEST_HIGHLIGHT_STYLE} )) || [[ -n "${functions[_zsh_autosuggest_suggest]}" ]]; then
  print "alias: zsh-autosuggestions detected. Ghost text may conflict." >&2
fi
