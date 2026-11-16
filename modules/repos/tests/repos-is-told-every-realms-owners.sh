# shellcheck shell=bash
# Reads the refresh's unit as home-manager wrote it and fails unless it
# gives repos every realm's owners as the one line it was asked to:
# systemd splits an unquoted Environment= at whitespace, so a value holding
# any would lose a realm's owners. modules/repos/checks.nix says why.
# usage: repos-is-told-every-realms-owners.sh <unit> <the Environment= line wanted>
set -euo pipefail

unit=$1
wanted=$2
if ! grep -qxF "$wanted" "$unit"; then
  echo "repos-refresh.service doesn't give every realm's owners as one word:" >&2
  grep -F REPOS_RECEPTOR "$unit" >&2 || echo "it gives no owners at all" >&2
  exit 1
fi
