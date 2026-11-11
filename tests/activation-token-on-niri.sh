# shellcheck shell=bash
# Starts a shell that asks niri for an activation token through
# Quickshell.activationToken (activation-token.qml, as the shell's root),
# and checks niri gave it one rather than nothing.
# usage: activation-token-on-niri.sh <shell folder> <logs>
set -euo pipefail

shell=$1
said=$2/shell.log
# How long, in tenths of a second, the shell may take to be given one.
saying=300

qs -p "$shell" > "$said" 2>&1 &
asking=$!
# shellcheck disable=SC2064 # The shell's pid is known now, and is what's ended.
trap "kill $asking 2> /dev/null || true" EXIT

for _ in $(seq "$saying"); do
  if grep -qF 'token[]' "$said"; then
    echo "niri gave the shell no token:" >&2
    cat "$said" >&2
    exit 1
  fi
  grep -q 'token\[[^]]' "$said" && exit 0
  sleep 0.1
done
echo "The shell was never given a token in $((saying / 10)) s:" >&2
cat "$said" >&2
exit 1
