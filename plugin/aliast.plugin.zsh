# aliast.plugin.zsh -- Ghost text suggestions via a Rust daemon
# Connects to aliast over a Unix socket, sends NDJSON requests on
# keystrokes, and renders ghost text after the cursor with POSTDISPLAY.

# ── 1. Guard and module loading ──────────────────────────────────────
(( ${+_ALIAST_LOADED} )) && return
typeset -g _ALIAST_LOADED=1

zmodload zsh/net/socket || {
  print "aliast: zsh/net/socket module required" >&2
  return 1
}
autoload -Uz add-zle-hook-widget

# ── 2. State variables ──────────────────────────────────────────────
typeset -g _ALIAST_FD=""
typeset -g _ALIAST_REQ_ID=0
typeset -g _ALIAST_HIGHLIGHT_ENTRY=""
typeset -g _ALIAST_LAST_BUFFER=""
typeset -g _ALIAST_LAST_HISTNUM=0
typeset -g _ALIAST_SOCKET_PATH="${XDG_RUNTIME_DIR:-/tmp/aliast-$UID}/aliast/aliast.sock"
typeset -g _ALIAST_NL_STATE="inactive"   # inactive | input | generating | review
typeset -g _ALIAST_NL_PROMPT=""           # Saved prompt for regeneration
typeset -g _ALIAST_LAST_EXIT=""
typeset -g _ALIAST_GIT_BRANCH=""
typeset -ga _ALIAST_PENDING_CMDS _ALIAST_PENDING_CWDS _ALIAST_PENDING_EXITS
typeset -g _ALIAST_INFLIGHT_ID=""   # id of the newest complete request (async)

# ── JSON helpers (pure -- no ZLE/I/O, unit-tested in tests/json_test.zsh) ──
# Results are returned in $REPLY to avoid a subprocess fork on the per-keystroke
# hot path (command substitution would fork a subshell every keystroke).

# Escape a raw string for embedding as a JSON string value.
_aliast_json_escape() {
  local s="$1"
  s="${s//\\/\\\\}"     # backslash -> \\
  s="${s//\"/\\\"}"     # "         -> \"
  s="${s//$'\n'/\\n}"   # newline   -> \n
  s="${s//$'\r'/\\r}"   # CR        -> \r
  s="${s//$'\t'/\\t}"   # tab       -> \t
  REPLY="$s"
}

# Reverse _aliast_json_escape for a JSON-escaped string value.
# Control chars are bound to locals because $'...' is taken literally in the
# replacement half of ${x//pat/repl} inside double quotes (it only expands in
# the pattern half).
_aliast_json_unescape() {
  local s="$1"
  local sentinel=$'\x01' nl=$'\n' cr=$'\r' tab=$'\t'
  s="${s//\\\\/$sentinel}"  # \\ -> sentinel (protect real backslashes first)
  s="${s//\\\"/\"}"         # \" -> "
  s="${s//\\n/$nl}"         # \n -> newline
  s="${s//\\r/$cr}"         # \r -> CR
  s="${s//\\t/$tab}"        # \t -> tab
  s="${s//\\\//\/}"         # \/ -> /
  s="${s//$sentinel/\\}"    # sentinel -> backslash
  REPLY="$s"
}

# Extract the "type" field from a response line (value has no escapes).
_aliast_response_type() { local r="${1##*\"type\":\"}"; REPLY="${r%%\"*}"; }

# Extract the "id" field from a response line (value has no escapes).
_aliast_response_id() { local r="${1##*\"id\":\"}"; REPLY="${r%%\"*}"; }

# Extract and unescape the trailing "text" field (suggestion/command responses).
# Responses are serde-internally-tagged, so "text" is always the final field.
_aliast_response_text() {
  local r="${1##*\"text\":\"}"
  r="${r%\"\}}"
  _aliast_json_unescape "$r"
}

# Extract and unescape the trailing "msg" field (error responses).
_aliast_response_msg() {
  local r="${1##*\"msg\":\"}"
  r="${r%\"\}}"
  _aliast_json_unescape "$r"
}

# True (0) when a generated command matches a destructive pattern that deserves
# a red tint in the NL review buffer before the user decides to run it.
_aliast_is_dangerous() {
  local cmd="$1"
  [[ "$cmd" =~ 'rm[[:space:]]+(-[a-zA-Z]*[rf][a-zA-Z]*[[:space:]]+)+' ]] && return 0
  [[ "$cmd" =~ '(^|[[:space:];&|])sudo[[:space:]]' ]] && return 0
  [[ "$cmd" =~ '(curl|wget)[^|;]*\|[^|]*(ba|z)?sh' ]] && return 0
  [[ "$cmd" =~ 'dd[[:space:]][^;|]*of=/dev/' ]] && return 0
  [[ "$cmd" =~ 'mkfs' ]] && return 0
  [[ "$cmd" =~ '>[[:space:]]*/dev/(sd|disk|rdisk)' ]] && return 0
  [[ "$cmd" =~ 'chmod[[:space:]]+(-[a-zA-Z]+[[:space:]]+)*777[[:space:]]+/([[:space:]]|$)' ]] && return 0
  return 1
}

# Tint the whole review buffer red when the generated command is dangerous.
# Uses the aliast-nl memo so existing NL cleanup paths remove it.
_aliast_nl_mark_danger() {
  if _aliast_is_dangerous "$BUFFER"; then
    region_highlight+=("0 $#BUFFER fg=red,bold memo=aliast-nl")
  fi
}

# Write one NDJSON request to a socket fd byte-exact. Plain `print` (like echo)
# processes backslash escapes, turning the JSON escape \" into a raw " on the
# wire -- the daemon then rejects any payload containing a quote. -r disables
# that; -- guards against a leading dash.
_aliast_send() {
  print -r -u $1 -- "$2"
}

# Drain a socket fd until a response with the wanted id and type arrives,
# discarding any stale/queued lines. Sets REPLY to the matched raw line and
# returns 0, or clears REPLY and returns 1 on timeout/EOF. Fork-free: safe on
# the per-keystroke hot path. This id match is what keeps ghost text from
# desyncing when a prior read timed out and left a response buffered.
_aliast_read_response() {
  local fd="$1" want_id="$2" want_type="$3" timeout="${4:-0.1}" line=""
  while read -r -u $fd -t $timeout line; do
    _aliast_response_type "$line"
    [[ "$REPLY" == "$want_type" ]] || continue
    _aliast_response_id "$line"
    if [[ "$REPLY" == "$want_id" ]]; then
      REPLY="$line"
      return 0
    fi
  done
  REPLY=""
  return 1
}

# ── 3. Connection management (lazy, non-blocking -- no polling, ever) ───
# A failed connect spawns the daemon fire-and-forget and returns immediately;
# the NEXT event (keystroke/precmd) picks up the connection once the daemon is
# up. This keeps the cold-start window free of per-keystroke stalls.
typeset -g _ALIAST_SPAWN_AT=-100

_aliast_connect() {
  # Already connected
  [[ -n "$_ALIAST_FD" ]] && return 0

  # Try connecting to existing daemon (instant when the socket is absent)
  zsocket "$_ALIAST_SOCKET_PATH" 2>/dev/null && {
    _ALIAST_FD=$REPLY
    return 0
  }

  # Daemon not running -- check if aliast binary is on PATH (avoids spawn attempts when missing)
  (( $+commands[aliast] )) || return 1

  # Respect an explicit `aliast stop`: while the marker exists, do not
  # auto-respawn (the precmd hook would otherwise resurrect the daemon before
  # the next prompt renders). `aliast start` clears the marker.
  [[ -e "${_ALIAST_SOCKET_PATH:h}/autostart-disabled" ]] && return 1

  # Spawn daemon (fire-and-forget), at most once every few seconds so a broken
  # install does not fork a launcher per keystroke. No waiting here.
  if (( SECONDS - _ALIAST_SPAWN_AT >= 5 )); then
    _ALIAST_SPAWN_AT=$SECONDS
    command aliast start &>/dev/null &!
  fi
  return 1
}

_aliast_disconnect() {
  [[ -z "$_ALIAST_FD" ]] && return
  zle -F "$_ALIAST_FD" 2>/dev/null   # unregister the async handler, if any
  exec {_ALIAST_FD}>&-
  _ALIAST_FD=""
  _ALIAST_INFLIGHT_ID=""
}

_aliast_reconnect() {
  _aliast_disconnect
  _aliast_connect
}

# ── 4. Ghost text rendering ─────────────────────────────────────────

# Resolve the style preset to a highlight spec, cached in $_ALIAST_STYLE.
# Priority: ALIAST_SUGGESTION_HIGHLIGHT (custom) > ALIAST_SUGGESTION_STYLE (preset) > default (dark).
# Called at load and per-prompt (precmd), never per-keystroke, so rendering does
# not fork a subshell on the hot path.
_aliast_resolve_style() {
  # Custom override always wins
  if [[ -n "$ALIAST_SUGGESTION_HIGHLIGHT" ]]; then
    typeset -g _ALIAST_STYLE="$ALIAST_SUGGESTION_HIGHLIGHT"
    return
  fi

  # Named presets using hex colors (terminal-palette-independent)
  case "${ALIAST_SUGGESTION_STYLE:-dark}" in
    dark)       typeset -g _ALIAST_STYLE="fg=#666666" ;;
    light)      typeset -g _ALIAST_STYLE="fg=#999999" ;;
    solarized)  typeset -g _ALIAST_STYLE="fg=#586e75" ;;
    *)          typeset -g _ALIAST_STYLE="fg=#666666" ;;
  esac
}
_aliast_resolve_style   # resolve once at load; refreshed per-prompt in precmd

_aliast_show_ghost() {
  local suggestion_text="$1"

  _aliast_clear_ghost

  POSTDISPLAY="$suggestion_text"

  if (( $#POSTDISPLAY > 0 )); then
    local start=$#BUFFER
    local end=$(( start + $#POSTDISPLAY ))
    _ALIAST_HIGHLIGHT_ENTRY="${start} ${end} ${_ALIAST_STYLE} memo=aliast"
    region_highlight+=("$_ALIAST_HIGHLIGHT_ENTRY")
  fi
}

_aliast_clear_ghost() {
  # Remove all aliast-owned highlight entries (memo=aliast)
  region_highlight=("${(@)region_highlight:#* memo=aliast}")
  _ALIAST_HIGHLIGHT_ENTRY=""
  POSTDISPLAY=""
}

# ── 5. IPC -- async: send the request, render when the reply arrives ─
# The keystroke path never reads the socket. A zle -F watcher fires when the
# daemon's reply lands, and renders it only if it is for the NEWEST request
# (id match) and the buffer has not changed since (stale-render guard). Typing
# latency is therefore independent of daemon latency.
_aliast_request_suggestion() {
  _aliast_connect || return

  (( _ALIAST_REQ_ID++ ))
  local req_id="r${_ALIAST_REQ_ID}"

  _aliast_json_escape "$BUFFER"; local escaped_buffer="$REPLY"
  _aliast_json_escape "$PWD"; local escaped_cwd="$REPLY"

  # Git branch: use cached value from precmd (per D-05)
  local branch_field=""
  if [[ -n "$_ALIAST_GIT_BRANCH" ]]; then
    _aliast_json_escape "$_ALIAST_GIT_BRANCH"
    branch_field=",\"git_branch\":\"${REPLY}\""
  fi

  local exit_field=""
  if [[ -n "$_ALIAST_LAST_EXIT" ]]; then
    exit_field=",\"exit_code\":${_ALIAST_LAST_EXIT}"
  fi

  local msg="{\"id\":\"${req_id}\",\"type\":\"complete\",\"buf\":\"${escaped_buffer}\",\"cur\":${CURSOR},\"cwd\":\"${escaped_cwd}\"${exit_field}${branch_field}}"

  _aliast_send $_ALIAST_FD "$msg" 2>/dev/null || {
    _aliast_reconnect
    return
  }

  typeset -g _ALIAST_INFLIGHT_ID="$req_id"
  typeset -g _ALIAST_INFLIGHT_BUF="$BUFFER"
  zle -F $_ALIAST_FD _aliast_fd_handler 2>/dev/null
}

# zle -F watcher: runs when the socket becomes readable while the line editor
# is waiting for input. Not a widget itself, so all editor-state changes go
# through the _aliast_render_ghost widget.
_aliast_fd_handler() {
  local fd="$1" line="" consumed=0
  while read -r -u $fd -t 0 line; do
    consumed=1
    _aliast_response_type "$line"
    [[ "$REPLY" == "suggestion" ]] || continue   # drop acks/stray frames
    _aliast_response_id "$line"
    [[ "$REPLY" == "$_ALIAST_INFLIGHT_ID" ]] || continue   # drop stale replies
    _aliast_response_text "$line"
    typeset -g _ALIAST_GHOST_PAYLOAD="$REPLY"
    zle _aliast_render_ghost 2>/dev/null
  done
  if (( ! consumed )); then
    # Readable but nothing to read: the daemon closed the connection. Drop the
    # fd (also unregisters this handler) so the next keystroke reconnects.
    _aliast_disconnect
  fi
  return 0
}

# Widget: apply the async suggestion, unless the buffer moved on since the
# request was sent (e.g. backspace, which triggers no new request).
_aliast_render_ghost() {
  if [[ "$BUFFER" != "$_ALIAST_INFLIGHT_BUF" ]]; then
    return 0
  fi
  if [[ -n "$_ALIAST_GHOST_PAYLOAD" ]]; then
    _aliast_show_ghost "$_ALIAST_GHOST_PAYLOAD"
  else
    _aliast_clear_ghost
  fi
  zle -R
}
zle -N _aliast_render_ghost

# ── 6. Widget wrappers (minimal) ────────────────────────────────────
# Use .self-insert / .accept-line (dot-prefixed builtins) to avoid
# infinite recursion when other plugins have already wrapped self-insert.
_aliast_self_insert_wrapper() {
  zle .self-insert "$@"
  # Skip ghost text suggestions during NL mode (PREDISPLAY conflicts)
  [[ "$_ALIAST_NL_STATE" != "inactive" ]] && return
  _aliast_request_suggestion
}
zle -N self-insert _aliast_self_insert_wrapper

_aliast_nl_aware_accept() {
  case "$_ALIAST_NL_STATE" in
    input)
      # Enter in NL input mode → generate command
      if [[ -z "$BUFFER" ]]; then
        return  # Empty prompt, do nothing
      fi
      _ALIAST_NL_STATE="generating"
      _aliast_nl_generate
      ;;
    review)
      # Enter on generated command → execute it, then stay in NL mode
      PREDISPLAY=""
      zle .accept-line
      # precmd will fire, then we re-enter NL input mode
      ;;
    *)
      # Normal mode — clear ghost, accept line
      _aliast_clear_ghost
      zle .accept-line
      ;;
  esac
}
zle -N accept-line _aliast_nl_aware_accept

# ── 7. Accept keybindings ───────────────────────────────────────────
_aliast_accept_suggestion() {
  if [[ -n "$POSTDISPLAY" ]]; then
    # Save ghost text, clear display, then update buffer
    local ghost="$POSTDISPLAY"
    _aliast_clear_ghost
    BUFFER="${BUFFER}${ghost}"
    CURSOR=$#BUFFER
  else
    zle expand-or-complete
  fi
}
zle -N _aliast_accept_suggestion
bindkey '^I' _aliast_accept_suggestion

_aliast_accept_word() {
  if [[ -n "$POSTDISPLAY" ]]; then
    local remaining="$POSTDISPLAY"
    local word=""
    # Grab leading whitespace + next word
    if [[ "$remaining" =~ '^[[:space:]]*[^[:space:]]+' ]]; then
      word="${MATCH}"
    fi
    _aliast_clear_ghost
    BUFFER="${BUFFER}${word}"
    CURSOR=$#BUFFER
    # Re-request suggestion for updated buffer
    _aliast_request_suggestion
  else
    zle .complete-word
  fi
}
zle -N _aliast_accept_word
bindkey '^[[Z' _aliast_accept_word
bindkey '\e[Z' _aliast_accept_word

# ── 8. Hook registration (non-conflicting) ──────────────────────────
_aliast_line_pre_redraw() {
  if [[ "$BUFFER" != "$_ALIAST_LAST_BUFFER" ]]; then
    _ALIAST_LAST_BUFFER="$BUFFER"
    if [[ -z "$BUFFER" ]]; then
      _aliast_clear_ghost
    fi
  fi
}
add-zle-hook-widget zle-line-pre-redraw _aliast_line_pre_redraw

# ── 9. NL Mode: Natural Language to Command ────────────────────────

_aliast_nl_set_indicator() {
  PREDISPLAY="● "
  region_highlight+=("P0 1 fg=blue,bold memo=aliast-nl")
}

_aliast_nl_deactivate() {
  _ALIAST_NL_STATE="inactive"
  _ALIAST_NL_PROMPT=""
  region_highlight=("${(@)region_highlight:#* memo=aliast-nl}")
  PREDISPLAY=""
  BUFFER=""
  CURSOR=0
  zle -R
}

_aliast_nl_generate() {
  local prompt="$BUFFER"
  [[ -z "$prompt" && -n "$_ALIAST_NL_PROMPT" ]] && prompt="$_ALIAST_NL_PROMPT"
  [[ -z "$prompt" ]] && { _aliast_nl_deactivate; return; }

  _ALIAST_NL_PROMPT="$prompt"
  BUFFER=""

  local tmpfile
  tmpfile=$(mktemp "${TMPDIR:-/tmp}/aliast-nl.XXXXXX")

  # Disable job control to suppress [1] pid / [1] + done noise
  setopt local_options no_monitor

  # Background: open NEW connection, send generate request, write response
  {
    zmodload zsh/net/socket 2>/dev/null
    local bg_fd
    zsocket "$_ALIAST_SOCKET_PATH" 2>/dev/null
    bg_fd=$REPLY
    if [[ -z "$bg_fd" ]]; then
      echo "error:connect" > "$tmpfile"
      exit 1
    fi

    (( _ALIAST_REQ_ID++ ))
    _aliast_json_escape "$prompt"; local escaped="$REPLY"
    _aliast_json_escape "$PWD"; local escaped_cwd="$REPLY"
    local branch_field=""
    if [[ -n "$_ALIAST_GIT_BRANCH" ]]; then
      _aliast_json_escape "$_ALIAST_GIT_BRANCH"
      branch_field=",\"git_branch\":\"${REPLY}\""
    fi
    local exit_field=""
    if [[ -n "$_ALIAST_LAST_EXIT" ]]; then
      exit_field=",\"exit_code\":${_ALIAST_LAST_EXIT}"
    fi

    local msg="{\"id\":\"r${_ALIAST_REQ_ID}\",\"type\":\"generate\",\"prompt\":\"${escaped}\",\"cwd\":\"${escaped_cwd}\"${exit_field}${branch_field}}"
    _aliast_send $bg_fd "$msg" 2>/dev/null || { echo "error:send" > "$tmpfile"; exec {bg_fd}>&-; exit 1; }

    local line=""
    # Longer than the daemon's backend timeout (30s) so a slow generation surfaces
    # the daemon's specific error rather than this generic client-side timeout.
    read -r -u $bg_fd -t 35 line 2>/dev/null || { echo "error:timeout" > "$tmpfile"; exec {bg_fd}>&-; exit 1; }
    print -r -- "$line" > "$tmpfile"   # -r: keep JSON \" escapes byte-exact
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
      _ALIAST_NL_STATE="input"
      _aliast_nl_set_indicator
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
    _aliast_nl_set_indicator
    case "$error_type" in
      connect)
        BUFFER="# Error: cannot connect to daemon"
        ;;
      send|timeout)
        BUFFER="# Error: daemon communication failed"
        ;;
    esac
    _ALIAST_NL_STATE="review"
    CURSOR=$#BUFFER
    zle -R
    return
  fi

  # Parse the JSON response robustly. No id match needed here: generate uses its
  # own short-lived connection, so the single buffered line is this response.
  _aliast_response_type "$result"
  local resp_type="$REPLY"
  if [[ "$resp_type" == "command" ]]; then
    _aliast_response_text "$result"
    _aliast_nl_set_indicator
    BUFFER="$REPLY"
    CURSOR=$#BUFFER
    _ALIAST_NL_STATE="review"
    _aliast_nl_mark_danger
    zle -R
  elif [[ "$resp_type" == "error" ]]; then
    _aliast_response_msg "$result"
    _aliast_nl_set_indicator
    BUFFER="# ${REPLY}"
    CURSOR=$#BUFFER
    _ALIAST_NL_STATE="review"
    zle -R
  else
    _aliast_nl_set_indicator
    BUFFER="# Error: unexpected response"
    _ALIAST_NL_STATE="review"
    CURSOR=$#BUFFER
    zle -R
  fi
}

_aliast_nl_toggle() {
  if [[ "$_ALIAST_NL_STATE" == "inactive" ]]; then
    # Toggle ON — enter NL mode
    _ALIAST_NL_STATE="input"
    _aliast_clear_ghost
    _aliast_nl_set_indicator
    BUFFER=""
    CURSOR=0
    zle -R
  else
    # Toggle OFF — exit NL mode entirely
    _aliast_nl_deactivate
  fi
}
zle -N _aliast_nl_toggle
# NL toggle key: Ctrl+Space by default, but not every terminal emits it --
# set ALIAST_NL_KEY (bindkey syntax, e.g. '^G' or '\ea') to rebind.
bindkey "${ALIAST_NL_KEY:-^ }" _aliast_nl_toggle

_aliast_nl_escape() {
  if [[ "$_ALIAST_NL_STATE" == "review" ]]; then
    # Escape in review → back to NL input (clear generated command)
    _ALIAST_NL_STATE="input"
    BUFFER=""
    CURSOR=0
    region_highlight=("${(@)region_highlight:#* memo=aliast-nl}")
    _aliast_nl_set_indicator
    zle -R
  elif [[ "$_ALIAST_NL_STATE" != "inactive" ]]; then
    # Escape in input → exit NL mode
    _aliast_nl_deactivate
  else
    zle send-break
  fi
}
zle -N _aliast_nl_escape
bindkey '\e' _aliast_nl_escape

# ── 10. Command recording (precmd hook -- registered FIRST) ────────
_aliast_precmd_record() {
  # CRITICAL: Capture exit code FIRST -- $? is overwritten by every command
  local last_exit_code=$?
  # Store for use by _aliast_request_suggestion and _aliast_nl_generate
  typeset -g _ALIAST_LAST_EXIT=$last_exit_code

  # Cache git branch per prompt (not per keystroke) per D-05 and Pitfall 4
  typeset -g _ALIAST_GIT_BRANCH=""
  _ALIAST_GIT_BRANCH="$(git rev-parse --abbrev-ref HEAD 2>/dev/null)"

  # Refresh the ghost-text style per prompt so config changes take effect without
  # a reload, while keeping the per-keystroke render path fork-free.
  _aliast_resolve_style

  # Get the most recent history entry number and command via fc
  local fc_out
  fc_out="$(fc -ln -1 2>/dev/null)" || return
  # fc -ln -1 outputs just the command (no number), with leading whitespace
  local cmd="${fc_out#"${fc_out%%[! ]*}"}"
  [[ -z "$cmd" ]] && return

  # Deduplicate using the command text itself
  [[ "$cmd" == "$_ALIAST_LAST_RECORDED" ]] && return
  typeset -g _ALIAST_LAST_RECORDED="$cmd"

  if ! _aliast_connect; then
    # Daemon still starting (cold-start window): buffer the record so it is not
    # lost, and flush on the next prompt once connected. Bounded so a dead
    # daemon cannot grow the arrays forever.
    if (( ${#_ALIAST_PENDING_CMDS} < 20 )); then
      _ALIAST_PENDING_CMDS+=("$cmd")
      _ALIAST_PENDING_CWDS+=("$PWD")
      _ALIAST_PENDING_EXITS+=("$last_exit_code")
    fi
    return
  fi

  # Flush any records buffered while the daemon was starting.
  if (( ${#_ALIAST_PENDING_CMDS} > 0 )); then
    local -i pending_index
    for (( pending_index = 1; pending_index <= ${#_ALIAST_PENDING_CMDS}; pending_index++ )); do
      _aliast_send_record \
        "${_ALIAST_PENDING_CMDS[pending_index]}" \
        "${_ALIAST_PENDING_CWDS[pending_index]}" \
        "${_ALIAST_PENDING_EXITS[pending_index]}" || break
    done
    _ALIAST_PENDING_CMDS=() _ALIAST_PENDING_CWDS=() _ALIAST_PENDING_EXITS=()
  fi

  _aliast_send_record "$cmd" "$PWD" "$last_exit_code"
}

# Send one record request and drain its Ack (id-matched, so a stale suggestion
# line cannot be mistaken for the ack, nor the ack pollute a suggestion read).
_aliast_send_record() {
  local record_cmd="$1" record_cwd="$2" record_exit="$3"
  _aliast_json_escape "$record_cmd"; local escaped_cmd="$REPLY"
  _aliast_json_escape "$record_cwd"; local escaped_cwd="$REPLY"

  (( _ALIAST_REQ_ID++ ))
  local req_id="r${_ALIAST_REQ_ID}"
  local msg="{\"id\":\"${req_id}\",\"type\":\"record\",\"cmd\":\"${escaped_cmd}\",\"cwd\":\"${escaped_cwd}\",\"exit_code\":${record_exit}}"

  _aliast_send $_ALIAST_FD "$msg" 2>/dev/null || { _aliast_reconnect; return 1 }
  _aliast_read_response "$_ALIAST_FD" "$req_id" "ack" 0.2
  return 0
}

autoload -Uz add-zsh-hook
add-zsh-hook precmd _aliast_precmd_record

# ── 11. NL mode persistence (re-enter after command execution) ─────
_aliast_nl_precmd() {
  # If we just executed a command from NL review state, re-enter NL input
  if [[ "$_ALIAST_NL_STATE" == "review" ]]; then
    _ALIAST_NL_STATE="input"
    _ALIAST_NL_PROMPT=""
    # PREDISPLAY will be set when ZLE starts for the next prompt
  fi
}
add-zsh-hook precmd _aliast_nl_precmd

# Re-apply NL indicator when ZLE starts a new line (after precmd)
_aliast_nl_line_init() {
  if [[ "$_ALIAST_NL_STATE" == "input" ]]; then
    region_highlight=("${(@)region_highlight:#* memo=aliast-nl}")
    _aliast_nl_set_indicator
  fi
}
add-zle-hook-widget zle-line-init _aliast_nl_line_init

# ── 12. Compatibility detection ─────────────────────────────────────
if (( ${+_ZSH_AUTOSUGGEST_HIGHLIGHT_STYLE} )) || [[ -n "${functions[_zsh_autosuggest_suggest]}" ]]; then
  print "aliast: zsh-autosuggestions detected. Ghost text may conflict." >&2
fi
