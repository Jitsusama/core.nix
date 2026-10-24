# shellcheck shell=bash
# Starts the shell's notices (notices.qml, in the account's shell folder) on
# the niri nested-niri.sh runs, sends a notice as a program does, and checks
# the shell drew its card on a layer niri lists as the notices'. Then a
# critical one, which stays until it's dealt with, is put away by what
# Super+Shift+N runs, and kept in the history unread.
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
if ! niri msg layers | grep -q notifications; then
  echo "niri lists no notices' layer:" >&2
  niri msg layers >&2
  exit 1
fi

notify-send --urgency=critical Standup "in five minutes"
says "report[shows card Standup]"
"$@"
says "report[gone notifications]"
# The history is written a moment after the cards go.
unread='"summary":"Standup"[^}]*"read":false'
for _ in $(seq "$saying"); do
  kept=$(cat "$XDG_STATE_HOME"/quickshell/by-shell/*/notices.json 2> /dev/null || true)
  grep -q "$unread" <<< "$kept" && exit 0
  sleep 0.1
done
echo "The notice put away isn't kept unread: $kept" >&2
exit 1
