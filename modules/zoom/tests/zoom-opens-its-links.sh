# shellcheck shell=bash
# Whether zoom-opens joins each of Zoom's own links in the web client: the
# meeting with its passcode, the meeting alone when there's none or it isn't
# plain, and the web client's home for nothing, a link naming no meeting or
# one that isn't Zoom's. The window is a stand-in that writes down what it
# was asked.
# usage: zoom-opens-its-links.sh OPENS
set -euo pipefail

opens=$1
here=$PWD

printf '#!%s\necho "$*" >> %s/asked\n' "$BASH" "$here" > window
chmod +x window

failures=0
# Opens with the arguments after $1 and says whether what was asked is $1.
expect() {
  local meant=$1
  shift
  : > asked
  "$BASH" "$opens" "$here/window" "$@" 2> /dev/null || true
  said=$(cat asked)
  if [ "$said" != "$meant" ]; then
    echo "${*:-nothing}: asked '$said', meant '$meant'"
    failures=$((failures + 1))
  fi
}

web=https://app.zoom.us/wc
expect "$web/join/81234567890?pwd=AbC123.1" \
  "zoommtg://zoom.us/join?action=join&confno=81234567890&pwd=AbC123.1"
expect "$web/join/81234567890?pwd=xyz" "zoomus://us02web.zoom.us/join?confno=81234567890&pwd=xyz"
expect "$web/join/81234567890" "zoommtg://zoom.us/join?action=join&confno=81234567890"
expect "$web/join/81234567890" "zoommtg://zoom.us/join?confno=81234567890&pwd=a%22b#frag"
expect "$web/join/81234567890?pwd=xyz" "zoommtg://zoom.us/join?pwd=xyz&confno=81234567890#frag"
expect "$web/home"
expect "$web/home" "zoommtg://zoom.us/start?action=start"
expect "$web/home" "zoommtg://zoom.us/join?confno=812;rm&pwd=xyz"
expect "$web/home" "zoommtg://zoom.us/join?confno=https://example.com"
expect "$web/home" "https://example.com/?confno=81234567890"

if [ "$failures" -gt 0 ]; then
  exit 1
fi
