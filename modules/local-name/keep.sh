# shellcheck shell=bash
# Sets a Mac's local name back to the one given when macOS has changed it,
# and says what it was, so a rename lasts minutes rather than until someone
# notices. A name already right is left alone, so nothing is written; that
# is what ends the run its own write starts, since writing the name changes
# the settings the job watches. It tries to set the name back at most once
# in five minutes, by the time STAMP holds: when a rename comes from a
# conflict that lasts, as when another machine answers to the name or macOS
# still hears its own answers, setting the name back makes macOS probe and
# rename again, and the two would otherwise trade the name every few
# seconds. A name it can't read is said at most once in five minutes too,
# by STAMP.unread, so a read that keeps failing doesn't fill its log.
# usage: keep.sh <name> <stamp>
set -euo pipefail

name=$1
stamp=$2

# Says $1, with the time it's said.
say() {
  local when
  when=$(date -u +%Y-%m-%dT%H:%M:%SZ)
  echo "${when} $1" >&2
}

# Sets age to how many seconds ago file $1 says it was written, or to nothing
# when it says no time or a time still to come. The time is the file's
# words, not its modification time, so the Mac's find and the test's agree
# on it to the second; and one to come, as when the clock was set back after
# it was written, counts as old rather than holding every set-back off until
# the clock catches up.
ago() {
  age=""
  local written now
  [[ -s $1 ]] || return 0
  written=$(<"$1")
  [[ ${written} =~ ^[0-9]+$ ]] || return 0
  now=$(date +%s)
  ((now >= written)) || return 0
  age=$((now - written))
}

# scutil fails saying "LocalHostName: not set" when there's no name. Any
# other failure, as when configd isn't answering yet, is said and left for
# the next run, rather than read as no name: that would only write the name
# given, but would say a rename that never was. Only what it prints on
# success is the name, so a warning beside it isn't read as part of it.
if ! now=$(scutil --get LocalHostName 2> /dev/null); then
  why=$(scutil --get LocalHostName 2>&1) || true
  if [[ ${why} != "LocalHostName: not set" ]]; then
    ago "${stamp}.unread"
    if [[ -z ${age} ]] || ((age >= 300)); then
      date +%s > "${stamp}.unread"
      say "couldn't read the local name: ${why}"
    fi
    exit 1
  fi
  now=""
fi
[[ ${now} != "${name}" ]] || exit 0
ago "${stamp}"
if [[ -n ${age} ]] && ((age < 300)); then
  exit 0
fi
# The time is kept before the set, so a set that keeps failing is tried, and
# said, once in five minutes rather than on every run.
date +%s > "${stamp}"
if ! scutil --set LocalHostName "${name}"; then
  say "couldn't set the local name back to ${name} from ${now:-unset}"
  exit 1
fi
say "the local name was ${now:-unset}, so it is ${name} again"
