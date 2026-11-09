# shellcheck shell=bash
# Runs qmllint on the arguments given, prints every message it found, and
# exits as qmllint did. Qt 6.12's qmllint exits 255 once any file has more
# warnings than --max-warnings, counting warnings and errors but not infos,
# yet its text report can leave out a warning it counted: the shell's bare
# reads in a binding were counted and never printed, so the check failed
# showing only infos. Its JSON report holds every message it counted, so
# this reads that and prints each in qmllint's own form. A file it was given
# but never read is said, as qmllint fails on it without a word.
# usage: qmllint-counted <qmllint's arguments>
set -uo pipefail

report=$(mktemp)
trap 'rm -f "$report"' EXIT

status=0
qmllint --json "$report" "$@" || status=$?

if [ -s "$report" ]; then
  jq -r '
    .files[]
    | if has("filename") | not then "Error: qmllint never read one of its files"
      else .filename as $file
        | .warnings[]
        | ({ info: "Info", warning: "Warning", critical: "Error" }[.type] // .type) as $kind
        | "\($kind): \($file):\(.line // 0):\(.column // 0): \(.message) [\(.id)]"
      end
  ' "$report"
fi

if [ "$status" -ne 0 ]; then
  echo "qmllint failed, exiting $status" >&2
fi
exit "$status"
