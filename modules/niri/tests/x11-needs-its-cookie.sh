# shellcheck shell=bash
# Runs on a nested niri (tests/nested-niri.sh) whose PATH holds the
# session's xwayland-satellite: an X11 client holding the cookie in
# $XAUTHORITY reaches the display niri opens, which starts the satellite and
# Xwayland, and one without it is refused. What the clients say goes in
# x11.log beside niri's, which nested-niri.sh prints last when this fails.
# usage: x11-needs-its-cookie.sh <niri's log>
set -euo pipefail

log=$1
exec 2> "$(dirname "$log")/x11.log"

for _ in $(seq 100); do
  display=$(grep -ao 'listening on X11 socket: :[0-9]*' "$log" | grep -o ':[0-9]*$' || true)
  [ -n "$display" ] && break
  sleep 0.1
done
[ -n "$display" ] || {
  echo "niri opened no X11 display" >&2
  exit 1
}

# The first client starts the satellite, and Xwayland behind it, so it waits
# while they do.
timeout 60 xdpyinfo -display "$display" > /dev/null || {
  echo "a client holding the cookie couldn't reach $display" >&2
  exit 1
}

if XAUTHORITY=/dev/null timeout 60 xdpyinfo -display "$display" > /dev/null 2>&1; then
  echo "a client without the cookie reached $display" >&2
  exit 1
fi
echo "$display lets in only the client holding its cookie"
