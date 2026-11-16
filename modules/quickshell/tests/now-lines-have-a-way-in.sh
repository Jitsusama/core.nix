# shellcheck shell=bash
# Fails on any piece of Now that can show a line with no way into more:
# no action Return runs, no drawer of its own or borrowed, no place it
# opens and no rows it holds. Joel found the Work Network saying a bare
# "degraded" he could do nothing with (2026-11-15: "if we are going to put
# something in now, it has to give me more information, someway to engage,
# link into more details"), so a line that says a state and stops fails.
# usage: now-lines-have-a-way-in.sh <pieces folder>
set -euo pipefail

# The pieces excused, each with why a way in would add nothing.
declare -A excused=(
  # Caps Lock's own key is the way out, and there's nothing more to say.
  [CapsLock.qml]=1
)

failed=0
for file in "$1"/[A-Z]*.qml; do
  name=$(basename "$file")
  # Piece.qml is the type every piece is, not a piece of its own.
  [ "$name" = Piece.qml ] && continue
  # A piece that never says active shows no line in Now.
  grep -qE '^\s*active\s*:' "$file" || continue
  if grep -qE '^\s*(actions|every|drawer|drawnBy|opens|rows|place|heldNames)\s*:|function (drawer|rows)\(' \
    "$file"; then
    continue
  fi
  [ -n "${excused[$name]:-}" ] && continue
  echo "$name shows a line in Now with no way in: give it actions, a drawer or a place it opens" >&2
  failed=1
done
exit "$failed"
