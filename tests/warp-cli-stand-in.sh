# shellcheck shell=bash
# warp-cli for the shell's checks, answering `warp-cli --json <question>`
# with what a test wrote for it in $HOME/warp-cli, named by the question's
# words joined by dashes: tunnel-stats.json is said as warp-cli says an
# answer, and tunnel-stats.error.json as it says an error, exiting 1. A
# question with nothing written is refused, as warp-cli refuses one its
# service won't answer.
set -euo pipefail

[ "${1:-}" = --json ] || {
  echo "the stand-in answers only --json" >&2
  exit 2
}
shift
question=$(
  IFS=-
  echo "$*"
)
said=$HOME/warp-cli/$question
if [ -e "$said.json" ]; then
  cat "$said.json"
elif [ -e "$said.error.json" ]; then
  cat "$said.error.json"
  exit 1
else
  echo '{"code": "NothingWritten", "error": "the test wrote no answer"}'
  exit 1
fi
