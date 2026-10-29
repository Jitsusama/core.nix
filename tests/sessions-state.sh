#!/usr/bin/env bash
# Lays out pi's sessions for tests/sessions.qml under the folder given, as
# the harness keeps them: a presence record for each, and the processes
# they name. Two run, started as kitty starts a window's program, one saying
# so in its record and one only in its environment; one runs where niri
# shows no window of its kitty; one's process has ended; one is from an
# earlier boot; and one names a running pid that started at another time,
# as a pid used again does. A stand-in kitten lists one kitty's windows,
# and a stand-in mycelium refuses a steer.
# The running processes' pids are written beside them, for the test to end.
set -euo pipefail

state=$1
presence=$state/pi/agentic-harness.pi/mesh-workflow/presence
mkdir -p "$presence" "$state/bin" "$state/mycelium"
# The realm a machine's node keeps, whose socket isn't there yet.
echo harbour >"$state/mycelium/realm"
boot=$(cat /proc/sys/kernel/random/boot_id)

# A process's start in clock ticks: the stat's 22nd field, counted after the
# name's closing bracket, which may hold spaces.
ticks() {
  local stat
  stat=$(cat "/proc/$1/stat")
  stat=${stat##*) }
  cut -d ' ' -f 20 <<<"$stat"
}

# A record: its file, its address, its owner's pid, boot and start, and any
# more fields, as JSON.
record() {
  printf '{"session":"s-%s","addresses":["%s"],"owner":{"pid":%s,"bootId":"%s","startTicks":%s}%s}\n' \
    "$2" "$2" "$3" "$4" "$5" "$6" >"$presence/$1.json"
}

KITTY_LISTEN_ON=unix:@kitty-500 KITTY_WINDOW_ID=3 sleep 300 &
live=$!
sleep 300 &
other=$!
KITTY_LISTEN_ON=unix:@kitty-700 KITTY_WINDOW_ID=2 sleep 300 &
unseen=$!
true &
ended=$!
wait "$ended"

record live LIVE01 "$live" "$boot" "$(ticks "$live")" \
  ',"busy":true,"title":"\u03c0 - Live One - core.nix"'
record other OTHER2 "$other" "$boot" "$(ticks "$other")" \
  ',"title":"\u03c0 - Lighthouse","terminal":{"driverId":"kitty","kind":"kitty-window","hostId":"here","value":"4","scope":"unix:@kitty-600"}'
record unseen NOWIN6 "$unseen" "$boot" "$(ticks "$unseen")" ''
record ended ENDED3 "$ended" "$boot" 1 ''
record old OLD004 "$live" another-boot "$(ticks "$live")" ''
record reused REUSE5 "$live" "$boot" "$(($(ticks "$live") + 1))" ''
echo "$live" >"$state/live.pid"

# What the last boot's end took away, as the harness keeps it: one whole,
# one whose folder is gone since, one with no folder and one whose id a
# shell would read.
cat >"$state/pi/restore.json" <<'EOF'
{"v":1,"boot":"an-earlier-boot","sessions":[
  {"id":"s-before","address":"BEFORE","title":"Harbour","cwd":"/tmp","lostAt":1},
  {"id":"s-given-back","address":"GIVENB","title":"Given Back","cwd":"/given-back","lostAt":1},
  {"id":"s-nowhere","address":"NOWHRE","title":"Nowhere","lostAt":1},
  {"id":"s; rm -rf ~","address":"SHELLY","title":"Shelly","cwd":"/tmp","lostAt":1}
]}
EOF
echo "$other" >"$state/other.pid"

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

# The realm's node refuses a steer to a machine out of reach, as mycelium
# says it, after keeping what it was asked.
cat >"$state/bin/mycelium" <<EOF
#!$BASH
[ "\$3" = steer ] || exit 1
{ echo "\$*"; cat; } >"$state/steered"
echo '{"Err":"lighthouse is out of reach, so FARAWY can'"'"'t be told now"}'
EOF
chmod +x "$state/bin/mycelium"
