#!/usr/bin/env bash
# Lays out what pi's sessions leave beside what the machine's node says of
# them, for tests/sessions.qml, under the folder given: the realm the node
# keeps, what the last boot's end took away, closed sessions' own records,
# and a stand-in kitten listing one kitty's windows. The sessions themselves
# are the node's, which the test stands in for, so no presence record is
# laid out and no process started.
set -euo pipefail

state=$1
mkdir -p "$state/pi/agentic-harness.pi" "$state/bin" "$state/mycelium"
# The realm a machine's node keeps, whose socket isn't there yet.
echo harbour >"$state/mycelium/realm"

# What the last boot's end took away, as the harness keeps it: one whole,
# one pi never saved, with its quest, one whose folder is gone since, one with no folder and one whose id a
# shell would read; and two windows joined to sessions another machine
# holds, one by its words as an argv and one by a line a shell would read;
# and three another machine carried here as it went away, two whole, the
# older listed first, and one saying where from badly; one pi's host
# here gave back, between them in time, one saying so badly and one
# saying it was both carried and given back; and one that ran hosted, which its host
# brings back itself, so reopening it starts no pi.
cat >"$state/pi/restore.json" <<'EOF'
{"v":1,"boot":"an-earlier-boot","sessions":[
  {"id":"s-before","address":"BEFORE","title":"Harbour","cwd":"/tmp","lostAt":1},
  {"id":"s-fresh","address":"FRESH7","title":"Fresh","cwd":"/tmp","lostAt":1,"fresh":true,
   "quest":"QEST-FRESH"},
  {"id":"s-given-back","address":"GIVENB","title":"Given Back","cwd":"/given-back","lostAt":1},
  {"id":"s-hosted","address":"HOSTED","title":"Hosted","cwd":"/tmp","lostAt":1,"hosted":true},
  {"id":"s-nowhere","address":"NOWHRE","title":"Nowhere","lostAt":1},
  {"id":"s; rm -rf ~","address":"SHELLY","title":"Shelly","cwd":"/tmp","lostAt":1},
  {"id":"s-far","address":"FARLST","title":"Far","cwd":"/tmp","lostAt":"",
   "machine":"lighthouse","attach":["ssh","-t","lighthouse","mycelium","attach","s-far"]},
  {"id":"s-said","address":"SAIDSO","title":"Said","cwd":"/tmp","lostAt":"",
   "machine":"lighthouse","attach":"ssh lighthouse mycelium attach s-said"},
  {"id":"s-ferried","address":"FERRID","title":"Ferried","cwd":"/tmp","lostAt":"",
   "carried":{"from":"lighthouse","at":"2026-10-30T10:00:00Z"}},
  {"id":"s-ferry","address":"FERRYQ","title":"Ferry","cwd":"/tmp","lostAt":"",
   "carried":{"from":"lighthouse","at":"2026-10-30T12:00:00Z"}},
  {"id":"s-adrift","address":"ADRIFT","title":"Adrift","cwd":"/tmp","lostAt":"",
   "carried":"lighthouse"},
  {"id":"s-freed","address":"FREEDQ","title":"Freed","cwd":"/tmp","lostAt":"",
   "released":{"from":"attended","at":"2026-10-30T11:00:00Z"}},
  {"id":"s-loose","address":"LOOSEQ","title":"Loose","cwd":"/tmp","lostAt":"",
   "released":"attended"},
  {"id":"s-twice","address":"TWICEQ","title":"Twice","cwd":"/tmp","lostAt":"",
   "carried":{"from":"lighthouse","at":"2026-10-30T09:00:00Z"},
   "released":{"from":"attended","at":"2026-10-30T09:00:00Z"}}
]}
EOF

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
