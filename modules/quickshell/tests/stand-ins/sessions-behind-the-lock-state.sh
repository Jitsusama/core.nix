#!/usr/bin/env bash
# Lays out what tests/sessions-behind-the-lock.qml needs beside the session
# it hands another beside, and tests/sessions-hosted-reopen.qml beside the
# hosted sessions it reopens, which each one's stand-in for the machine's
# node tells of, under the folder given: the realm the node keeps, and the
# list pi's host keeps of the sessions it runs, which names s-viewed alone.
set -euo pipefail

state=$1
mkdir -p "$state/pi/agentic-harness.pi/hosts" "$state/mycelium"
echo harbour >"$state/mycelium/realm"
cat >"$state/pi/agentic-harness.pi/hosts/4100-7.json" <<'EOF'
{"key":"harbour-background","owner":{"pid":4100,"startedAt":17,"bootId":"b1","startTicks":7},
 "sessions":[{"id":"s-viewed","state":"idle","since":"2026-11-15T10:00:00Z"}]}
EOF
