# shellcheck shell=bash
# Starts the shell's notices (notices.qml, in the account's shell folder) on
# the headless sway nested-niri.sh runs niri in, rather than on niri, since
# nested niri's one output never goes while sway's can be unplugged and made
# again. With a notice up, the only screen goes, a second notice comes while
# there's none, and a screen comes back; Quickshell must warn of nothing all
# along, where on the net's vm-screens-keep-the-launcher it warned of a
# dangling screen object each time the last screen went.
# usage: notices-lose-their-screen.sh <shell folder> <logs> <shell-says.sh>
set -euo pipefail

shell=$1
logs=$2
# shellcheck source=SCRIPTDIR/lib/shell-says.sh
source "$3"
said=$logs/shell.log
# How long, in tenths of a second, the shell may take to say each thing.
saying=300

# sway's own socket, which nested-niri.sh started it on.
sockets=("${XDG_RUNTIME_DIR:?}"/sway-ipc.*.sock)
export SWAYSOCK=${sockets[0]}

WAYLAND_DISPLAY=wayland-1 qs -p "$shell" > "$said" 2>&1 &
drawing=$!
# shellcheck disable=SC2064 # The shell's pid is known now, and is what's ended.
trap "kill $drawing 2> /dev/null || true" EXIT

says "Configuration Loaded"
notify-send --urgency=critical --app-name=Check "A notice" "Up as the screen goes."
says "report[lines notice A notice]"
says "report[drawn notifications]"

swaymsg output HEADLESS-1 unplug
says "report[gone notifications]"
notify-send --urgency=critical --app-name=Check "A second notice" "Sent with no screen."
says "report[shows card A second notice]"
qs -p "$shell" ipc call notices front
sleep 1

swaymsg create_output
# The line is drawn again once there's a screen to stand on.
before=$(grep -c -F "report[drawn notifications]" "$said")
for _ in $(seq "$saying"); do
  drawn=$(grep -c -F "report[drawn notifications]" "$said")
  [[ $drawn -gt $before ]] && break
  sleep 0.1
done
qs -p "$shell" ipc call notices front

if grep -a -F -e WARN -e dangling "$said"; then
  echo "Quickshell warned as its screen went and came." >&2
  cat "$said" >&2
  exit 1
fi
cat "$said"
