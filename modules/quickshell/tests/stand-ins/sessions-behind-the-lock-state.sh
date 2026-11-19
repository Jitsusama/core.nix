#!/usr/bin/env bash
# Lays out what modules/quickshell/tests/sessions-behind-the-lock.qml needs
# beside the session it hands another beside, and
# modules/quickshell/tests/sessions-hosted-reopen.qml beside the hosted
# sessions it reopens, which each one's stand-in for the machine's node tells
# of, under the folder given: the realm the node keeps, the list pi's host
# keeps of the sessions it runs, which names s-viewed alone, and the harness's
# quest index, saying where a new session on QEST-ADRIFT starts, for the one
# whose folder is gone.
set -euo pipefail

state=$1
mkdir -p "$state/pi/agentic-harness.pi/hosts" "$state/pi/agentic-harness.pi/quest-workflow" \
  "$state/mycelium" "$state/trees/keel"
echo harbour >"$state/mycelium/realm"
cat >"$state/pi/agentic-harness.pi/hosts/4100-7.json" <<'EOF'
{"key":"harbour-background","owner":{"pid":4100,"startedAt":17,"bootId":"b1","startTicks":7},
 "sessions":[{"id":"s-viewed","state":"idle","since":"2026-11-15T10:00:00Z"}]}
EOF
printf '{"quests":[{"id":"QEST-ADRIFT","start":"%s"}]}\n' "$state/trees/keel" \
  >"$state/pi/agentic-harness.pi/quest-workflow/quests.json"
