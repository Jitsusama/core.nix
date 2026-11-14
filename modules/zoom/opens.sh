# shellcheck shell=bash
# Opens Zoom's web client in a window of its own, on a machine Zoom builds
# no client for. A meeting's page hands the browser Zoom's own link
# (zoommtg: or zoomus:), which names the meeting and its passcode, so that
# link joins the meeting in the web client, as Omarchy's handler does;
# anything else, or nothing, opens the web client's home. Only a meeting's
# number and a plain passcode are passed on, so a link can't steer the
# window anywhere but Zoom.
# usage: opens.sh WINDOW [LINK], WINDOW opening a URL in a window of its
# own, as jitsusama.zoom.window says. Its help takes the shape every
# command of ours does (docs/commands.md), by hand.
set -euo pipefail

window=$1
link=${2-}

case "$link" in
-h | --help)
  cat << 'EOF'
zoom-opens  open Zoom's web client in a window of its own

  A meeting's zoommtg: or zoomus: link joins the meeting,
  with its passcode; anything else, or nothing, opens the
  web client's home.

EXAMPLES
  zoom-opens 'zoommtg://zoom.us/join?confno=123&pwd=ab'
  zoom-opens

EXIT
  0   opened, else the window's own
EOF
  exit 0
  ;;
*) ;;
esac
web=https://app.zoom.us/wc

query=
case "$link" in
zoommtg://*\?* | zoomus://*\?*)
  query=${link#*\?}
  query=${query%%#*}
  ;;
esac

meeting=
passcode=
IFS='&' read -r -a fields <<< "$query"
for field in "${fields[@]}"; do
  case "$field" in
  confno=*) meeting=${field#confno=} ;;
  pwd=*) passcode=${field#pwd=} ;;
  esac
done

if [[ ! $meeting =~ ^[0-9]+$ ]]; then
  exec "$window" "$web/home"
fi
if [[ $passcode =~ ^[A-Za-z0-9._-]+$ ]]; then
  exec "$window" "$web/join/$meeting?pwd=$passcode"
fi
exec "$window" "$web/join/$meeting"
