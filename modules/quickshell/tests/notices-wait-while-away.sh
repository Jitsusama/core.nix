# shellcheck shell=bash
# Starts the shell's notices (notices.qml, in the account's shell folder) on
# the niri tests/lib/nested-niri.sh runs, with Joel counted away after a second
# of nothing typed or moved. Nothing types here, so he's away before a notice
# comes; the notice must then stay past the time its sender gave it. Nothing
# here can bring him back (docs/testing.md, The Desktop on niri), so a notice
# counting again once he is stays the desktop test's.
# usage: notices-wait-while-away.sh <shell folder> <logs> <shell-says.sh>
set -euo pipefail

shell=$1
logs=$2
# shellcheck source=SCRIPTDIR/../../../tests/lib/shell-says.sh
source "$3"
said=$logs/shell.log
# How long, in tenths of a second, the shell may take to say each thing.
saying=300
# How long the notice is sent to last, in milliseconds, and how long, in
# seconds, it must stay all the same while Joel's away: three times that.
lasting=1000
staying=3

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
if grep -qF -e "report[rests While away]" -e "report[gone notifications]" "$said"; then
  echo "The notice counted down while Joel was away:" >&2
  cat "$said" >&2
  exit 1
fi
# The line's layer stays mapped while it's quiet too, so the shell says
# whether it still shows the notice.
shows=$(qs -p "$shell" ipc call notices shows)
if [ "$shows" != notice ]; then
  echo "The arrival line shows '$shows' rather than the notice while Joel's away." >&2
  exit 1
fi
