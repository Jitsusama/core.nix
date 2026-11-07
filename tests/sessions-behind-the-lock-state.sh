#!/usr/bin/env bash
# Lays out what tests/sessions-behind-the-lock.qml needs beside the session
# it hands another beside, which its stand-in for the machine's node tells
# of, under the folder given: the realm the node keeps. A stand-in kitten says kitty 600's two windows when first asked, and its
# three only once the test writes them to the fifo kitty-600-later, so the
# test decides when kitty has said which window is whose. A third ask would
# wait on the fifo with no writer, which the check's timeout ends; the test
# changes kitty's windows only once, so it asks only twice.
set -euo pipefail

state=$1
mkdir -p "$state/pi/agentic-harness.pi" "$state/bin" "$state/mycelium"
echo harbour >"$state/mycelium/realm"

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
