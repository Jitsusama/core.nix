#!/usr/bin/env bash
# Lays out what pi's sessions leave beside what the machine's node says of
# them, for tests/sessions.qml, under the folder given: the realm the node
# keeps, closed sessions' own records, and a stand-in kitten listing one
# kitty's windows. What the last boot's end took away the agents daemon
# serves, which the test stands in for too. The sessions themselves
# are the node's, which the test stands in for, so no presence record is
# laid out and no process started.
set -euo pipefail

state=$1
mkdir -p "$state/pi/agentic-harness.pi" "$state/bin" "$state/mycelium"
# The realm a machine's node keeps, whose socket isn't there yet.
echo harbour >"$state/mycelium/realm"

# Closed sessions' own records, as pi keeps them, each opening with the
# folder it ran in: one in a tree still there, which isn't the home
# folder, and one in a tree given back since.
sessions=$HOME/.pi/agent/sessions/--trees--
mkdir -p "$sessions" "$state/trees/karst"
printf '{"type":"session","id":"s-kept","cwd":"%s"}\n' "$state/trees/karst" \
  >"$sessions/2026-10-23T17-53-44-653Z_s-kept.jsonl"
printf '{"type":"session","id":"s-gone","cwd":"%s"}\n' "$state/trees/given-back" \
  >"$sessions/2026-09-29T17-54-04-184Z_s-gone.jsonl"

# kitty 600 has two windows, the session's the second.
cat >"$state/bin/kitten" <<EOF
#!$BASH
echo "\$*" >>"$state/kitten-said"
if [ "\$3" = unix:@kitty-600 ] && [ "\$4" = ls ]; then
  echo '[{"id":2,"tabs":[{"windows":[{"id":4}]}]},{"id":1,"tabs":[{"windows":[{"id":1}]}]}]'
fi
EOF
chmod +x "$state/bin/kitten"
