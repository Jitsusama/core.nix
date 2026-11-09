# shellcheck shell=bash
# Writes a session log as pi writes one, for spend-watch to import: a
# session started 25 hours ago in /src/thing, then a turn on opus and one a
# minute before it on sonnet each hour since, every turn the same tokens and
# the same $1.50.
# usage: spend-session.sh <log>
set -euo pipefail

log=$1
now=$(date -u +%s)
# A time this many seconds ago, as pi writes one.
at() {
  date -u -d "@$((now - $1))" +%FT%TZ
}
# A turn, at a time and on a model.
turn() {
  printf '{"type":"message","timestamp":"%s","message":{"role":"assistant","model":"%s",' "$1" "$2"
  printf '"usage":{"input":10,"output":100,"cacheRead":1000,"cacheWrite":100,'
  printf '"cost":{"total":1.5}}}}\n'
}

mkdir -p "$(dirname "$log")"
printf '{"type":"session","version":3,"id":"s1","timestamp":"%s","cwd":"/src/thing"}\n' \
  "$(at 90000)" > "$log"
for hours in $(seq 24 -1 0); do
  turn "$(at $((hours * 3600)))" opus >> "$log"
  turn "$(at $((hours * 3600 + 60)))" sonnet >> "$log"
done
