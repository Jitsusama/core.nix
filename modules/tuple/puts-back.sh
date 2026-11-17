# shellcheck shell=bash
# Puts our Tuple desktop entry back when it no longer reads as ours, hidden
# and opening tuple: links, as after a daemon started outside tuple.service
# writes its own. Any generation's entry reads as ours, so an old
# generation's unit never takes back a new one home-manager has just
# linked, and its own putting back doesn't set it off again. It links into
# home-manager's current generation, as home-manager does, so the link is
# home-manager's to replace or clean up, and to the entry it was built with
# only while there's no generation to link into.
# usage: puts-back.sh <entry> <generation's files> <entry in them> <ours>
set -euo pipefail

entry=$1
generation=$2
inGeneration=$3
ours=$4

if grep -qx NoDisplay=true "$entry" 2>/dev/null &&
  grep -qx 'MimeType=x-scheme-handler/tuple;' "$entry"; then
  exit 0
fi
linked=$(readlink -f "$generation" || true)$inGeneration
[ -f "$linked" ] || linked=$ours
ln -sfn "$linked" "$entry"
echo "Tuple's own desktop entry was in the place of ours; ours is back"
