# shellcheck shell=bash
# Sets a Mac's local name back to the one given when macOS has changed it,
# and says what it was, so a rename lasts minutes rather than until someone
# notices. A name already right is left alone, so nothing is written; that
# is what ends the run its own write starts, since writing the name changes
# the settings the job watches. It sets the name back at most once in five
# minutes, by STAMP's time: when a rename comes from a conflict that lasts,
# as when another machine answers to the name or macOS still hears its own
# answers, setting the name back makes macOS probe and rename again, and
# the two would otherwise trade the name every few seconds.
# usage: keep.sh <name> <stamp>
set -euo pipefail

name=$1
stamp=$2
when=$(date -u +%Y-%m-%dT%H:%M:%SZ)
# scutil fails saying "not set" when there's no name. Any other failure, as
# when configd isn't answering yet, is said and left for the next run, rather
# than read as no name and written over a name that may be right.
if ! now=$(scutil --get LocalHostName 2>&1); then
  if [[ ${now} != *"not set"* ]]; then
    echo "${when} couldn't read the local name: ${now}" >&2
    exit 1
  fi
  now=""
fi
[[ ${now} != "${name}" ]] || exit 0
recent=$(find "${stamp}" -mmin -5 2>/dev/null) || recent=""
[[ -z ${recent} ]] || exit 0
touch "${stamp}"
if ! scutil --set LocalHostName "${name}"; then
  echo "${when} couldn't set the local name back to ${name} from ${now:-unset}" >&2
  exit 1
fi
echo "${when} the local name was ${now:-unset}, so it is ${name} again" >&2
