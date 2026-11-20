#!/usr/bin/env bash
# The body of a-theme-changes-everything: a home built on another theme keeps
# none of Osaka Jade's colours. It takes a file of Osaka Jade's colours, one
# to a line, and the paths to read.
#
#   a-theme-changes-everything.sh <colours file> <path>...
set -euo pipefail

colours=$1
shift

# An empty list of colours would match nothing, and pass whatever the home
# holds; a path that isn't there would be read as a link that can't be
# followed, and pass unread.
if ! test -s "$colours"; then
  echo "There are no colours to look for in $colours."
  exit 1
fi
for path in "$@"; do
  if ! test -e "$path"; then
    echo "$path isn't there to read."
    exit 1
  fi
done

said=$(mktemp -d)
trap 'rm -rf "$said"' EXIT

# rg ends in 2 on any error, a match found or not, and a home links files
# outside the sandbox that it can't follow; so what it found is read from
# what it printed, and an end other than 1 passes only when every error it
# said is such a link.
status=0
rg --follow --ignore-case --fixed-strings --line-number --no-heading --hidden \
  --glob '!**/skills/aesthetic/design.md' \
  --glob '!**/skills/web-design/examples/**' \
  --glob '!**/jitsusama/web/theme.css' \
  --file "$colours" "$@" > "$said/found" 2> "$said/unread" || status=$?

if [ -s "$said/found" ]; then
  cat "$said/found"
  echo "The files above kept Osaka Jade's colours when the theme changed."
  exit 1
fi

if [ "$status" = 1 ]; then
  exit 0
fi
if [ "$status" = 2 ] && [ -s "$said/unread" ] \
  && ! grep -v 'No such file or directory' "$said/unread"; then
  exit 0
fi
echo "rg ended in $status without reading every file."
exit 1
