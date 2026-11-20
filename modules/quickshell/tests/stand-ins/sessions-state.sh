#!/usr/bin/env bash
# Lays out what pi's sessions leave beside what the machine's node says of
# them, for modules/quickshell/tests/sessions.qml, under the folder given: the
# realm the node keeps, closed sessions' own records, and a stand-in kitten
# listing one kitty's windows. What the last boot's end took away the agents
# daemon serves, which the test stands in for too. The sessions themselves are
# the node's, which the test stands in for, so no presence record is laid out
# and no process started.
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
# One parked whose holder died, which the node can't wake, so Wake reopens it.
printf '{"type":"session","id":"s-orphan","cwd":"%s"}\n' "$state/trees/karst" \
  >"$sessions/2026-11-17T21-02-11-301Z_s-orphan.jsonl"
# And one fenced strict, which Wake reopens by its id under its own fence.
printf '{"type":"session","id":"s-warded","cwd":"%s"}\n' "$state/trees/karst" \
  >"$sessions/2026-11-17T21-03-40-118Z_s-warded.jsonl"
# And one fenced by the fence's own word, reopened under it, and one whose
# presence names no fence, which keeps its file and is still refused.
printf '{"type":"session","id":"s-hedged","cwd":"%s"}\n' "$state/trees/karst" \
  >"$sessions/2026-11-17T21-03-41-118Z_s-hedged.jsonl"
printf '{"type":"session","id":"s-unsaid","cwd":"%s"}\n' "$state/trees/karst" \
  >"$sessions/2026-11-17T21-03-42-118Z_s-unsaid.jsonl"
# And three Wake must never reopen, though each has its file: one parked
# elsewhere, one fenced by a word no fence goes by, and one running here
# already.
for id in s-stray s-fenced s-LIVE01; do
  printf '{"type":"session","id":"%s","cwd":"%s"}\n' "$id" "$state/trees/karst" \
    >"$sessions/2026-11-17T21-02-12-000Z_$id.jsonl"
done

# kitty 600 has two windows, the session's the second.
cat >"$state/bin/kitten" <<EOF
#!$BASH
echo "\$*" >>"$state/kitten-said"
if [ "\$3" = unix:@kitty-600 ] && [ "\$4" = ls ]; then
  echo '[{"id":2,"tabs":[{"windows":[{"id":4}]}]},{"id":1,"tabs":[{"windows":[{"id":1}]}]}]'
fi
EOF
chmod +x "$state/bin/kitten"
