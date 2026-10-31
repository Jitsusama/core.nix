# shellcheck shell=bash
# Starts the shell's notices (notices.qml, in the account's shell folder) on
# the niri nested-niri.sh runs, sends a notice as a program does, and checks
# the shell drew its card on a layer niri lists as the notices'. Then a
# critical one, which stays until it's dealt with, is put away by what
# Super+Shift+N runs, and kept in the history unread. And a sender whose
# notices are missed until seen keeps one that answers what Joel just did,
# said by the x-answers hint, unmissed, as Google Drive's saves are.
# usage: notices-on-niri.sh <shell folder> <logs> <what Super+Shift+N runs>...
set -euo pipefail

shell=$1
logs=$2
shift 2
said=$logs/shell.log
# How long, in tenths of a second, the shell may take to say each thing.
saying=300

# Waits until the shell has said something, or fails saying what it never
# said.
says() {
  local tries
  for tries in $(seq "$saying"); do
    grep -qF "$1" "$said" && return 0
    sleep 0.1
  done
  echo "The shell never said '$1' in $((tries / 10)) s." >&2
  exit 1
}

qs -p "$shell" > "$said" 2>&1 &
drawing=$!
# shellcheck disable=SC2064 # The shell's pid is known now, and is what's ended.
trap "kill $drawing 2> /dev/null || true" EXIT

says "Configuration Loaded"
notify-send --app-name=Check "A notice" "Sent as a program sends one."
says "report[shows card A notice]"
# The card is made before its window is drawn, and niri lists the layer
# once it has the window's first frame, which a loaded machine can take a
# while to send.
says "report[drawn notifications]"
listed=false
layers=""
for _ in $(seq "$saying"); do
  # Read whole before it's matched: piped into grep -q, niri could still be
  # writing when grep stops at the match, and its broken pipe would fail the
  # pipeline under pipefail, as if the layer were missing.
  layers=$(niri msg layers)
  if grep -q notifications <<< "$layers"; then
    listed=true
    break
  fi
  sleep 0.1
done
if [ "$listed" != true ]; then
  echo "niri lists no notices' layer:" >&2
  echo "$layers" >&2
  exit 1
fi

notify-send --app-name="Google Drive" --hint=string:x-answers:save \
  "Saved to Google Drive" "Notes.txt, in My Drive"
says "report[shows card Saved to Google Drive]"
notify-send --app-name="Google Drive" "Not Saved to Google Drive" "Notes.txt: Google signed out"
says "report[shows card Not Saved to Google Drive]"

notify-send --urgency=critical Standup "in five minutes"
says "report[shows card Standup]"
"$@"
says "report[gone notifications]"
# The history is written a moment after the cards go.
unread='"summary":"Standup"[^}]*"read":false'
answered='"summary":"Saved to Google Drive"[^}]*"missed":false'
missed='"summary":"Not Saved to Google Drive"[^}]*"missed":true'
for _ in $(seq "$saying"); do
  kept=$(cat "$XDG_STATE_HOME"/quickshell/by-shell/*/notices.json 2> /dev/null || true)
  if grep -q "$unread" <<< "$kept" && grep -q "$answered" <<< "$kept" \
    && grep -q "$missed" <<< "$kept"; then
    exit 0
  fi
  sleep 0.1
done
echo "The history doesn't keep the notice put away unread, the answer unmissed and the" \
  "failure missed: $kept" >&2
exit 1
