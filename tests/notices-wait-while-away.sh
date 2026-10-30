# shellcheck shell=bash
# Starts the shell's notices (notices.qml, in the account's shell folder) on
# the niri nested-niri.sh runs, with Joel counted away after a second of
# nothing typed or moved. Nothing types here, so he's away before a notice
# comes; the notice must then stay past the time its sender gave it. Nothing
# here can bring him back (docs/testing.md, The Shell on niri), so a notice
# counting again once he is stays the desktop test's.
# usage: notices-wait-while-away.sh <shell folder> <logs>
set -euo pipefail

shell=$1
logs=$2
said=$logs/shell.log
# How long, in tenths of a second, the shell may take to say each thing.
saying=300
# How long the notice is sent to last, in milliseconds, and how long, in
# seconds, it must stay all the same while Joel's away: three times that.
lasting=1000
staying=3

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
says "report[away]"
notify-send --app-name=Check --expire-time="$lasting" "While away" "Still here when he's back."
says "report[shows card While away]"
# Long past its time, it's still on screen and never started counting.
sleep "$staying"
if grep -qF -e "report[counts card While away]" -e "report[gone notifications]" "$said"; then
  echo "The notice counted down while Joel was away:" >&2
  cat "$said" >&2
  exit 1
fi
# Read whole before it's matched: piped into grep -q, niri could still be
# writing when grep stops at the match, and its broken pipe would fail the
# pipeline under pipefail, as if the layer were missing.
layers=$(niri msg layers)
if ! grep -q notifications <<< "$layers"; then
  echo "niri lists no notices' layer while Joel's away:" >&2
  echo "$layers" >&2
  exit 1
fi
