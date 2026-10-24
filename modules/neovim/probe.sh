# shellcheck shell=bash
# Runs Neovim headless with the arguments given, its probe writing its
# answer to its output, and fails with what went wrong unless it answered
# as expected and complained of nothing. One that never finishes is
# stopped after two minutes, so a check fails rather than waits.
# usage: probe.sh <expected answer> <nvim argument>...
set -euo pipefail

expected=$1
shift
timeout 120 nvim --headless "$@" > answer 2> complaints || true
if [ -s complaints ]; then
  cat complaints
  exit 1
fi
if [ ! -s answer ]; then
  echo "Neovim gave no answer, or didn't finish in two minutes"
  exit 1
fi
if [ "$(cat answer)" != "$expected" ]; then
  cat answer
  exit 1
fi
