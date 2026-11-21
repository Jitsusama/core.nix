# shellcheck shell=bash
# Sets a Mac's local name back to the one given when macOS has changed it,
# and says what it was, so a rename lasts seconds rather than until someone
# notices. A name already right is left alone, so nothing is written.
# usage: keep.sh <name>
set -euo pipefail

name=$1
now=$(scutil --get LocalHostName 2>/dev/null) || now=""
if [[ "${now}" != "${name}" ]]; then
  scutil --set LocalHostName "${name}"
  when=$(date -u +%Y-%m-%dT%H:%M:%SZ)
  echo "${when} the local name was ${now:-unset}, so it is ${name} again" >&2
fi
