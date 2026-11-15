# shellcheck shell=bash
# Sourced by a check that starts the shell with its output in a log, `said`,
# and gives it `saying` tenths of a second to say each thing.

# Waits until the shell has said something, or fails saying what it never
# said.
says() {
  local tries
  for tries in $(seq "${saying:?}"); do
    grep -qF "$1" "${said:?}" && return 0
    sleep 0.1
  done
  echo "The shell never said '$1' in $((tries / 10)) s." >&2
  exit 1
}
