#!/usr/bin/env zsh
# Unit tests for the aliast plugin's pure JSON helpers.
#
# These helpers must be side-effect free (no ZLE, no I/O) so they can be unit
# tested. We stub the interactive builtins/functions the plugin wires up at load
# time, then source the plugin and exercise the pure functions directly.
emulate -L zsh

# Stub interactive commands so sourcing the plugin does not touch ZLE or fpath.
autoload() { return 0 }
zmodload() { return 0 }
zle() { return 0 }
bindkey() { return 0 }
add-zsh-hook() { return 0 }
add-zle-hook-widget() { return 0 }

typeset -g PLUGIN_DIR="${0:A:h:h}"
typeset -g _ALIAST_UNIT_TEST=1
source "$PLUGIN_DIR/aliast.plugin.zsh"

typeset -gi _fail=0 _count=0
check() {
  local desc="$1" expected="$2" actual="$3"
  (( _count++ ))
  if [[ "$expected" == "$actual" ]]; then
    print -r -- "ok   $_count - $desc"
  else
    print -r -- "FAIL $_count - $desc"
    print -r -- "        expected |$expected|"
    print -r -- "        actual   |$actual|"
    (( _fail++ ))
  fi
}

# --- escape (building requests) ---
_aliast_json_escape 'git commit -m "fix"'
check "escape double quotes" 'git commit -m \"fix\"' "$REPLY"

_aliast_json_escape 'C:\temp'
check "escape backslash" 'C:\\temp' "$REPLY"

_aliast_json_escape $'a\nb\tc'
check "escape newline and tab" 'a\nb\tc' "$REPLY"

# --- unescape (parsing responses) ---
_aliast_json_unescape 'git commit -m \"fix\"'
check "unescape double quotes" 'git commit -m "fix"' "$REPLY"

_aliast_json_unescape 'C:\\temp'
check "unescape backslash" 'C:\temp' "$REPLY"

_aliast_json_unescape 'a\nb'
check "unescape newline" $'a\nb' "$REPLY"

# An escaped backslash followed by n must NOT become a newline.
_aliast_json_unescape 'a\\nb'
check "unescape escaped-backslash before n" 'a\nb' "$REPLY"

# --- roundtrip ---
local original='echo "hi\there"; cd C:\x'
_aliast_json_escape "$original"; local esc="$REPLY"
_aliast_json_unescape "$esc"
check "escape/unescape roundtrip" "$original" "$REPLY"

# --- response field extraction ---
local sug='{"type":"suggestion","id":"r7","text":"ls -la"}'
_aliast_response_type "$sug"; check "type extract" "suggestion" "$REPLY"
_aliast_response_id "$sug";   check "id extract" "r7" "$REPLY"
_aliast_response_text "$sug"; check "text extract" "ls -la" "$REPLY"

local q='{"type":"command","id":"r9","text":"git commit -m \"fix\""}'
_aliast_response_text "$q"; check "text extract with embedded quotes" 'git commit -m "fix"' "$REPLY"

local empty='{"type":"suggestion","id":"r1","text":""}'
_aliast_response_text "$empty"; check "empty text extract" "" "$REPLY"

local err='{"type":"error","id":"r3","msg":"bad thing \"x\""}'
_aliast_response_type "$err"; check "error type extract" "error" "$REPLY"
_aliast_response_msg "$err";  check "error msg extract with quotes" 'bad thing "x"' "$REPLY"

local ack='{"type":"ack","id":"r5"}'
_aliast_response_type "$ack"; check "ack type extract" "ack" "$REPLY"
_aliast_response_id "$ack";   check "ack id extract" "r5" "$REPLY"

# --- wire writes must be byte-exact (plain `print` processes \" -> ") ---
local wire_tmp="$(mktemp)" wfd
exec {wfd}> "$wire_tmp"
_aliast_send $wfd '{"cmd":"echo \"hi\" C:\\temp"}'
exec {wfd}>&-
check "send preserves escapes byte-exact" '{"cmd":"echo \"hi\" C:\\temp"}' "$(<"$wire_tmp")"
rm -f "$wire_tmp"

# --- id-matched response draining (the desync fix) ---
# A stale response buffered ahead of the fresh one must be discarded, not rendered.
local drain_tmp="$(mktemp)"
print -r -- '{"type":"suggestion","id":"r1","text":"stale one"}' >> "$drain_tmp"
print -r -- '{"type":"suggestion","id":"r2","text":"fresh two"}' >> "$drain_tmp"
local dfd
exec {dfd}< "$drain_tmp"
_aliast_read_response "$dfd" "r2" "suggestion" 0.1; local rc=$?
check "drain finds matching id" 0 "$rc"
_aliast_response_text "$REPLY"
check "drain skips stale, returns fresh" "fresh two" "$REPLY"
exec {dfd}<&-

# No matching id -> non-zero, empty REPLY.
exec {dfd}< "$drain_tmp"
_aliast_read_response "$dfd" "r9" "suggestion" 0.1; rc=$?
check "drain returns non-zero when no id matches" 1 "$rc"
check "drain leaves REPLY empty when no match" "" "$REPLY"
exec {dfd}<&-
rm -f "$drain_tmp"

# --- async render guard (stale replies must not paint over a changed buffer) ---
typeset -g BUFFER='git ch' POSTDISPLAY='' _ALIAST_INFLIGHT_BUF='git ch'
typeset -g _ALIAST_GHOST_PAYLOAD='eckout main'
typeset -ga region_highlight
_aliast_render_ghost
check "async render applies when buffer unchanged" "eckout main" "$POSTDISPLAY"

BUFFER='git c'   # simulated backspace after the request went out
_ALIAST_GHOST_PAYLOAD='SHOULD NOT RENDER'
_aliast_render_ghost
check "async render skipped when buffer changed" "eckout main" "$POSTDISPLAY"

BUFFER='git ch'; _ALIAST_INFLIGHT_BUF='git ch'; _ALIAST_GHOST_PAYLOAD=''
_aliast_render_ghost
check "async render clears ghost on empty payload" "" "$POSTDISPLAY"

print -r -- "---"
print -r -- "$(( _count - _fail ))/$_count passed"
exit $(( _fail > 0 ? 1 : 0 ))
