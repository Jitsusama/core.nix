# shellcheck shell=bash
# Starts the account's whole shell (its shell folder's shell.qml) on the
# headless sway tests/lib/nested-niri.sh runs niri in, rather than on niri, since
# nested niri's one output never goes while sway's can be unplugged and made
# again. With a notice up on two screens, both go at once, as an undock takes
# them, a second notice comes while there's none, and a screen comes back;
# Quickshell must never warn of a dangling screen object, as it did on the
# net's vm-screens-keep-the-launcher as its two screens went, nor of a
# binding or handler that throws. The shell's notices alone (notices.qml)
# warned of neither on sway, so it's the whole shell that's run.
# usage: shell-loses-its-screen.sh <shell folder> <logs> <shell-says.sh>
set -euo pipefail

shell=$1
logs=$2
# shellcheck source=SCRIPTDIR/../../../tests/lib/shell-says.sh
source "$3"
said=$logs/shell.log
# How long, in tenths of a second, the shell may take to say each thing.
saying=300

# sway's own socket, which tests/lib/nested-niri.sh started it on.
sockets=("${XDG_RUNTIME_DIR:?}"/sway-ipc.*.sock)
export SWAYSOCK=${sockets[0]}

# The shell is drawn on sway, and asked over IPC there too, since qs finds
# only the instances on its own display.
export WAYLAND_DISPLAY=wayland-1
qs -p "$shell" > "$said" 2>&1 &
drawing=$!
# shellcheck disable=SC2064 # The shell's pid is known now, and is what's ended.
trap "kill $drawing 2> /dev/null || true" EXIT

says "Configuration Loaded"
# A second screen, so one goes while the other is still there, and what
# held the first is still about as the second goes.
swaymsg create_output
says "launcher heard 2 screens"
notify-send --urgency=critical --app-name=Check "A notice" "Up as the screen goes."
says "report[lines notice A notice]"
says "report[drawn notifications]"

swaymsg "output HEADLESS-1 unplug; output HEADLESS-2 unplug"
says "report[gone notifications]"
notify-send --urgency=critical --app-name=Check "A second notice" "Sent with no screen."
says "report[shows card A second notice]"
# Asked where the line is with no screen to stand on, which may fail: only
# what Quickshell warns of while it's asked is checked.
qs -p "$shell" ipc call notices front || true
# A second for any warning the ask gives to reach the log: a failed ask says
# nothing the shell's log could be waited on for.
sleep 1

# The line is drawn again once there's a screen to stand on.
before=$(grep -c -F "report[drawn notifications]" "$said")
swaymsg create_output
for _ in $(seq "$saying"); do
  drawn=$(grep -c -F "report[drawn notifications]" "$said")
  [[ $drawn -gt $before ]] && break
  sleep 0.1
done
qs -p "$shell" ipc call notices front

# Only a dangling screen and what throws are looked for: on sway the shell
# also warns of what this setup lacks (niri's background effect, the
# windows daemon and the sockets of daemons nothing starts here).
if grep -a -F -e dangling -e TypeError "$said"; then
  echo "Quickshell warned of a dangling screen, or threw, as its screen went and came." >&2
  cat "$said" >&2
  exit 1
fi
cat "$said"
