#!/usr/bin/env bash
# Lays out the session tests/sessions-behind-the-lock.qml hands another
# beside, under the folder given, as the harness keeps it: its presence
# record, naming the second window of kitty 600, and the process it names.
# A stand-in kitten says kitty 600's two windows when first asked, and its
# three only once the test writes them to the fifo kitty-600-later, so the
# test decides when kitty has said which window is whose.
set -euo pipefail

state=$1
presence=$state/pi/agentic-harness.pi/mesh-workflow/presence
mkdir -p "$presence" "$state/bin"
boot=$(cat /proc/sys/kernel/random/boot_id)

sleep 300 &
sender=$!
stat=$(cat "/proc/$sender/stat")
stat=${stat##*) }
ticks=$(cut -d ' ' -f 20 <<<"$stat")
owner=$(printf '{"pid":%s,"bootId":"%s","startTicks":%s}' "$sender" "$boot" "$ticks")
terminal='{"driverId":"kitty","kind":"kitty-window","hostId":"here","value":"4"'
terminal+=',"scope":"unix:@kitty-600"}'
printf '{"session":"s-sender","addresses":["SENDER"],"owner":%s,"terminal":%s}\n' \
  "$owner" "$terminal" >"$presence/sender.json"
echo "$sender" >"$state/sender.pid"

mkfifo "$state/kitty-600-later"
cat >"$state/bin/kitten" <<EOF
#!$BASH
[ "\$3" = unix:@kitty-600 ] && [ "\$4" = ls ] || exit 0
if mkdir "$state/kitty-600-asked" 2>/dev/null; then
  echo '[{"id":2,"tabs":[{"windows":[{"id":4}]}]},{"id":1,"tabs":[{"windows":[{"id":1}]}]}]'
else
  cat "$state/kitty-600-later"
fi
EOF
chmod +x "$state/bin/kitten"
