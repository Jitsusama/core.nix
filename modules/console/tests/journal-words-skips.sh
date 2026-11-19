# shellcheck shell=bash
# Checks that the journalctl a machine runs still words each journal file
# it skips as a failed boot's summary reads it (modules/console/stopped/
# src/main.rs's SKIPPING and SKIPPING_HINT). Were systemd to reword them,
# a skip would go back to being taken for one unit's reason, and nothing
# else would say so. The wordings are journal-util.c's, one per kind of
# skip: truncated, corrupted, an unknown feature, too many files, and any
# other error opening a file.
# usage: journal-words-skips.sh <main.rs> <systemd> ...
set -euo pipefail

main=$1
shift
# The constant's value, between the quotes on its own line.
constant() {
  sed -n "s/^const $1: &str = \"\\(.*\\)\";\$/\\1/p" "$main"
}
skipping=$(constant SKIPPING)
hint=$(constant SKIPPING_HINT)
[[ -n $skipping && -n $hint ]] || {
  echo "journal-words-skips: main.rs no longer names SKIPPING and SKIPPING_HINT" >&2
  exit 1
}

kinds=(
  "is truncated$skipping."
  "corrupted$skipping."
  "uses an unsupported feature$skipping."
  "in scope$skipping"
  "$skipping: %m"
)
for systemd; do
  shopt -s nullglob
  shared=("$systemd"/lib/systemd/libsystemd-shared-*.so)
  shopt -u nullglob
  ((${#shared[@]} == 1)) || {
    echo "journal-words-skips: no one libsystemd-shared in $systemd" >&2
    exit 1
  }
  for words in "${kinds[@]}" "$hint"; do
    grep -q -a -F -- "$words" "${shared[0]}" || {
      echo "journal-words-skips: $systemd's journalctl no longer says \"$words\"," >&2
      echo "so main.rs's SKIPPING or SKIPPING_HINT must follow its new words" >&2
      exit 1
    }
  done
done
