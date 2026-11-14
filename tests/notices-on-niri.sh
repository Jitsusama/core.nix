# shellcheck shell=bash
# Starts the shell's notices (notices.qml, in the account's shell folder) on
# the niri nested-niri.sh runs, sends a notice as a program does, and checks
# the shell drew its card on a layer niri lists as the notices', and showed
# it on the arrival line, an overlay layer that takes no keys and is mapped
# only while there's a notice to show. Then a
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
# The arrival line is mapped only while it shows a notice, so a quiet
# desktop has nothing of it to draw.
line='any(.[]; .namespace == "notice-line")'
if niri msg --json layers | jq -e "$line" > /dev/null; then
  echo "The arrival line is mapped with no notice to show." >&2
  exit 1
fi
notify-send --app-name=Check "A notice" "Sent as a program sends one."
says "report[shows card A notice]"
# It shows on the arrival line, an overlay layer that never takes the keys.
says "report[lines notice A notice]"
landed=false
for _ in $(seq "$saying"); do
  if niri msg --json layers | jq -e 'any(.[]; .namespace == "notice-line"
      and .layer == "Overlay" and .keyboard_interactivity == "None")' > /dev/null; then
    landed=true
    break
  fi
  sleep 0.1
done
if [ "$landed" != true ]; then
  echo "niri lists no arrival line on the overlay layer without the keys:" >&2
  niri msg --json layers >&2
  exit 1
fi
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
# Read before Super+Shift+N below, which counts every missed one seen.
# What the notices daemon keeps (modules/notices), read as each changes.
kept_file=$XDG_STATE_HOME/notices/notices.json
answered='any(.notices[]; .summary == "Saved to Google Drive" and (.missed | not))'
missed='any(.notices[]; .summary == "Not Saved to Google Drive" and .missed)'
held_apart=false
for _ in $(seq "$saying"); do
  kept=$(cat "$kept_file" 2> /dev/null || true)
  if jq -e "($answered) and ($missed)" <<< "$kept" > /dev/null 2>&1; then
    held_apart=true
    break
  fi
  sleep 0.1
done
if [ "$held_apart" != true ]; then
  echo "The history doesn't keep the answer unmissed and the failure missed: $kept" >&2
  exit 1
fi

notify-send --urgency=critical Standup "in five minutes"
says "report[shows card Standup]"
"$@"
says "report[gone notifications]"
# Every notice put away, the arrival line goes with them.
says "report[unlines notice"
for _ in $(seq "$saying"); do
  niri msg --json layers | jq -e "$line" > /dev/null || break
  sleep 0.1
done
if niri msg --json layers | jq -e "$line" > /dev/null; then
  echo "The arrival line is still mapped with every notice put away." >&2
  exit 1
fi
# The history is written a moment after the cards go.
unread='any(.notices[]; .summary == "Standup" and (.read | not))'
for _ in $(seq "$saying"); do
  kept=$(cat "$kept_file" 2> /dev/null || true)
  jq -e "$unread" <<< "$kept" > /dev/null 2>&1 && exit 0
  sleep 0.1
done
echo "The notice put away isn't kept unread: $kept" >&2
exit 1
