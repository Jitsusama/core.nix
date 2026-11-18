# shellcheck shell=bash
# warp-cli for the shell's checks, answering `warp-cli --json <question>`
# with what a test wrote for it in $HOME/warp-cli, named by the question's
# words joined by dashes: tunnel-stats.json is said as warp-cli says an
# answer, and tunnel-stats.error.json as it says an error, exiting 1. A
# question with nothing written is refused, as warp-cli refuses one its
# service won't answer. Each question asked is written to asked there, so
# a test sees what the shell had warp ask, and a connect or disconnect
# answered changes the status it says next, as WARP's would. It switches
# nothing: no WARP is ever asked.
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
if [ -d "$HOME/warp-cli" ]; then
  echo "$question" >>"$HOME/warp-cli/asked"
fi
if [ -e "$said.json" ]; then
  cat "$said.json"
  case $question in
  connect) state='"Connected", "reason": "NetworkHealthy"' ;;
  disconnect) state='"Disconnected", "reason": "ManualDisconnection"' ;;
  *) state= ;;
  esac
  if [ -n "$state" ]; then
    echo "{\"status\": $state}" >"$HOME/warp-cli/status.json"
  fi
elif [ -e "$said.error.json" ]; then
  cat "$said.error.json"
  exit 1
else
  echo '{"code": "NothingWritten", "error": "the test wrote no answer"}'
  exit 1
fi
