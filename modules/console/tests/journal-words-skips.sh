# shellcheck shell=bash
# Checks that the journalctl a failed boot's summary runs still words each
# journal file it skips as the summary reads it (modules/console/stopped/
# src/main.rs's SKIPPING and SKIPPING_HINT). Were systemd to reword them,
# a skip would go back to being taken for one unit's reason, and nothing
# else would say so. Each wording is journal-util.c's, matched as the whole
# string libsystemd-shared holds, so none can pass on another message's
# words: truncated, corrupted, an unknown feature with the hint on a line
# of its own under it, too many files, and any other error opening a file.
# usage: journal-words-skips.sh <main.rs> <systemd> ...
set -euo pipefail

main=$1
shift
(($# > 0)) || {
  echo "journal-words-skips: no systemd to read" >&2
  exit 1
}
# The constant's value, between the quotes on its own line.
constant() {
  sed -n "s/^const $1: &str = \"\\(.*\\)\";\$/\\1/p" "$main"
}
skipping=$(constant SKIPPING)
hint=$(constant SKIPPING_HINT)
if [[ -z $skipping || -z $hint || $skipping$hint == *$'\n'* ]]; then
  echo "journal-words-skips: main.rs no longer names SKIPPING and SKIPPING_HINT once each" >&2
  exit 1
fi

kinds=(
  "Journal file %s is truncated$skipping."
  "Journal file %s corrupted$skipping."
  "Journal file %1\$s uses an unsupported feature$skipping."$'\n'"$hint%1\$s to see the details."
  "Too many journal files (limit is at %u) in scope$skipping '%s'."
  "An error was encountered while opening journal file or directory %s$skipping: %m"
)
# A wording as a Perl pattern for the whole of one string in the library:
# each line taken literally, and the lines joined by a newline.
whole() {
  local pattern=${1//$'\n'/\\E\\n\\Q}
  printf '\\A\\Q%s\\E\\z' "$pattern"
}
for systemd; do
  shopt -s nullglob
  shared=("$systemd"/lib/systemd/libsystemd-shared-*.so)
  shopt -u nullglob
  ((${#shared[@]} == 1)) || {
    echo "journal-words-skips: no one libsystemd-shared in $systemd" >&2
    exit 1
  }
  for words in "${kinds[@]}"; do
    pattern=$(whole "$words")
    grep -q -a -z -P -- "$pattern" "${shared[0]}" || {
      echo "journal-words-skips: $systemd's journalctl no longer says, word for word:" >&2
      echo "  $words" >&2
      echo "so its journal-util.c words a skip anew: check that main.rs's SKIPPING" >&2
      echo "and SKIPPING_HINT still find it, then follow its words here" >&2
      exit 1
    }
  done
done
