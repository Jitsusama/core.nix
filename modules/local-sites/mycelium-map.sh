# shellcheck shell=bash
# Writes the mycelium site's now.json from what the realm's node says now,
# whole or not at all: each of the four answers is read to a file of its own,
# and only when all four came back is the map shaped, written beside the
# page under a temporary name and moved into place, so the page never reads
# half a file. When the node doesn't answer, as while it restarts, the last
# good map stays, one plain line says why, and the run still ends well: the
# page says how old its map is, and a unit failing every 20 seconds would
# say nothing more than that.
# usage: mycelium-map.sh <filter> <site folder>
# MYCELIUM_REALM names the realm, as for mycelium itself, and
# MYCELIUM_MAP_SOCKET a node's socket other than the realm's.
set -euo pipefail

filter=$1
site=$2
mkdir -p "$site"
work=$(mktemp -d "$site/.now.XXXXXX")
trap 'rm -rf "$work"' EXIT
ask=(mycelium)
if [[ -n ${MYCELIUM_MAP_SOCKET:-} ]]; then
  ask+=(--socket "$MYCELIUM_MAP_SOCKET")
fi

for view in sessions "swarm --json" machines messages; do
  name=${view%% *}
  # shellcheck disable=SC2086 # a view's words are meant to split
  if ! "${ask[@]}" $view > "$work/$name" 2> "$work/$name.said"; then
    said=$(< "$work/$name.said")
    said=${said//$'\n'/ }
    echo "mycelium didn't answer '$view' (${said:0:200}); the last map stays"
    exit 0
  fi
done

jq -n \
  --slurpfile s "$work/sessions" \
  --slurpfile w "$work/swarm" \
  --slurpfile m "$work/machines" \
  --slurpfile g "$work/messages" \
  -f "$filter" > "$work/now.json"
chmod 644 "$work/now.json"
mv "$work/now.json" "$site/now.json"
