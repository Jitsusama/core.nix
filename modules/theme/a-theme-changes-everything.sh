#!/usr/bin/env bash
# The body of a-theme-changes-everything: a home built on another theme keeps
# none of Osaka Jade's colours. It takes the theme's background, which proves
# the theme changed, a file of Osaka Jade's colours, one to a line, and the
# paths to read.
#
#   a-theme-changes-everything.sh <background> <colours file> <path>...
set -euo pipefail

background=$1
colours=$2
shift 2

test "$background" = '#FFFCF0'
# An empty list of colours would match nothing, and pass whatever the home
# holds.
test -s "$colours"

# rg ends in 2 on any error, a match found or not, and a home links files
# outside the sandbox that it can't follow; so what it found is read from
# what it printed, and only an error of another kind fails.
status=0
rg --follow --ignore-case --fixed-strings --line-number --no-heading --hidden \
  --glob '!**/skills/aesthetic/design.md' \
  --glob '!**/skills/web-design/examples/**' \
  --glob '!**/jitsusama/web/theme.css' \
  --file "$colours" "$@" > found 2> unread || status=$?

if [ -s found ]; then
  cat found
  echo "The files above kept Osaka Jade's colours when the theme changed."
  exit 1
fi

if [ "$status" != 1 ] && grep -v 'No such file or directory' unread; then
  echo "rg couldn't read the files above."
  exit 1
fi
