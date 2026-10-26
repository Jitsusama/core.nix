# shellcheck shell=bash
# Opens a link from Joel's calendar the way it's meant to be opened: a
# Tuple call through Tuple, where Tuple is installed, so it joins without a
# browser tab in between; Google's own pages (Meet, Calendar, Mail) in a
# window of their own, so each is a program on the desktop and a Meet call
# is a window niri can name; anything else in the default browser.
# usage: opens.sh WINDOW OPEN URL, WINDOW opening a URL in a window of its
# own, as jitsusama.google.window says, and OPEN in the default browser.
set -euo pipefail

window=$1
open=$2
url=$3

case "$url" in
https://tuple.app/c/* | https://production.tuple.app/c/*)
  if command -v tuple > /dev/null; then
    exec tuple call join "$url"
  fi
  exec "$open" "$url"
  ;;
https://meet.google.com/* | https://calendar.google.com/* | https://mail.google.com/*)
  exec "$window" "$url"
  ;;
https://* | http://*)
  exec "$open" "$url"
  ;;
*)
  echo "google-opens: not a link: $url" >&2
  exit 2
  ;;
esac
