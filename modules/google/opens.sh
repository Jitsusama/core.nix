# shellcheck shell=bash
# Opens a link from Joel's calendar the way it's meant to be opened: a
# Tuple call through Tuple, where Tuple is installed, so it joins without a
# browser tab in between; Google's own pages (Meet, Calendar, Mail, and
# Docs, Sheets, Slides and Drive) in a window of their own, so each is a
# program on the desktop and a Meet call is a window niri can name;
# anything else in the default browser.
# usage: opens.sh WINDOW OPEN URL, WINDOW opening a URL in a window of its
# own, as jitsusama.google.window says, and OPEN in the default browser.
# Its help and failures take the shape every command of ours does
# (docs/commands.md), by hand.
set -euo pipefail

window=$1
open=$2
url=${3:-}

case "$url" in
-h | --help)
  cat << 'EOF'
google-opens  opens a calendar link as it's meant to open

EXAMPLES
  google-opens URL   a Tuple call in Tuple, Google's pages
                     in their own window, the rest in the
                     browser

EXIT
  0   done
  64  asked wrongly
EOF
  exit 0
  ;;
https://tuple.app/c/* | https://production.tuple.app/c/*)
  if command -v tuple > /dev/null; then
    exec tuple call join "$url"
  fi
  exec "$open" "$url"
  ;;
https://meet.google.com/* | https://calendar.google.com/* | https://mail.google.com/* | \
  https://docs.google.com/* | https://drive.google.com/*)
  exec "$window" "$url"
  ;;
https://* | http://*)
  exec "$open" "$url"
  ;;
*)
  printf '%s\n' "google-opens opens only links, not ${url:-nothing}" \
    "  try  google-opens https://meet.google.com/..." >&2
  exit 64
  ;;
esac
