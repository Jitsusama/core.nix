# shellcheck shell=bash
# Reads the refresh's unit as home-manager wrote it and fails unless it
# gives repos every realm's owners as the one line it was asked to:
# systemd splits an unquoted Environment= at whitespace, so a value holding
# any would lose a realm's owners. It also follows the unit to the program
# it runs and fails unless that sets the declared login outright, since a
# refresh keeps whose review each change asks against it, and a default a
# caller could set, or a program run bare, would keep no review as asked.
# modules/repos/checks.nix says why.
# usage: repos-is-told-every-realms-owners.sh <unit> <the Environment= line wanted>
#        <the wrapper's line setting the login>
set -euo pipefail

unit=$1
wanted=$2
login=$3
if ! grep -qxF "$wanted" "$unit"; then
  echo "repos-refresh.service doesn't give every realm's owners as one word:" >&2
  grep -F REPOS_RECEPTOR "$unit" >&2 || echo "it gives no owners at all" >&2
  exit 1
fi
program=$(sed -n 's/^ExecStart=\([^ ]*\) refresh$/\1/p' "$unit")
if [[ -z $program ]] || ! grep -qxF "$login" "$program"; then
  echo "repos-refresh.service runs a repos that doesn't set the declared login:" >&2
  grep -F ExecStart "$unit" >&2
  if [[ -n $program ]]; then
    grep -F REPOS_LOGIN "$program" >&2 || echo "it says nothing of REPOS_LOGIN" >&2
  fi
  exit 1
fi
