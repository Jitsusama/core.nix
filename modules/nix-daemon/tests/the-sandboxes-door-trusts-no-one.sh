# shellcheck shell=bash
# Runs the untrusted door's own daemon command, as its socket unit runs it,
# over a store of its own in the build's folder, and asks it through the
# socket whether it trusts the caller. It must answer, and answer no.
#
# Arguments: systemd-socket-activate, the nix a client runs, and the door's
# command, word by word.
set -euo pipefail

activate=$1
nix=$2
shift 2

here=$(pwd -P)
socket=$here/door.sock

# The door's daemon gets only the environment named, as under systemd, so
# it keeps its store in the build's folder and reads no nix.conf.
HOME=$here NIX_REMOTE="local?root=$here/store" NIX_CONF_DIR=$here \
  "$activate" --accept --inetd -l "$socket" -E HOME -E NIX_REMOTE -E NIX_CONF_DIR "$@" \
  2>"$here/door.log" &
door=$!
trap 'kill "$door" 2>/dev/null || true' EXIT

for _ in $(seq 50); do
  [[ -S $socket ]] && break
  sleep 0.1
done

if ! answer=$(HOME=$here NIX_CONF_DIR=$here "$nix" --extra-experimental-features nix-command \
  --store "unix://$socket" store info 2>&1); then
  printf 'The door never answered:\n%s\n%s\n' "$answer" "$(cat "$here/door.log")" >&2
  exit 1
fi
if ! grep -qx 'Trusted: 0' <<<"$answer"; then
  printf 'The door trusts its caller, so a sandbox passing it passes root:\n%s\n' "$answer" >&2
  exit 1
fi
