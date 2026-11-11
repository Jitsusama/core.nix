# shellcheck shell=bash
# Fails on a call, in any module's QML or JavaScript, to a method Quickshell's
# JavaScript lacks, which qmllint lets through and the shell fails on only as
# it runs. Spotify's Escape script runs in Spotify's own browser, which has
# them, so it alone is left out.
# usage: the-desktop-speaks-its-javascript.sh <tree>
set -euo pipefail

cd "$1"
# ripgrep answers 0 for a match, 1 for none and 2 when it can't search, and
# only none passes.
status=0
rg --line-number --no-heading --glob '*.qml' --glob '*.js' \
  --glob '!modules/spotify/backs-out.js' \
  -e '\.(at|flatMap|flat|findLast|findLastIndex|toSorted|toReversed|toSpliced)\(' \
  -e '\b(structuredClone|Object\.groupBy|Object\.hasOwn|Object\.fromEntries)\(' \
  modules || status=$?
if [[ ${status} = 0 ]]; then
  echo "The lines above call a method Quickshell's JavaScript doesn't have."
  exit 1
fi
[[ ${status} = 1 ]]
