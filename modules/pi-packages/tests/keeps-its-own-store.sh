#!/usr/bin/env bash
# Proves pi-packages installs into the folder the agent fence holds
# read-only, and that pnpm, given the environment a build is given, keeps
# its store and its metadata cache inside it, and node-gyp its headers,
# never in the account's store or ~/.cache, which fenced commands write.
# pi loads every dependency from that store, so one a fenced command could
# rewrite would run outside the fence. pnpm itself is asked where its
# store and cache lie, so a name pnpm no longer reads fails here rather
# than passing; node-gyp's headers folder is checked by the value given.
#
# Usage: keeps-its-own-store.sh ROOT HELD EXECSTART
#   ROOT       the root pi-packages is configured with
#   HELD       the folder the fence holds read-only
#   EXECSTART  the service's command, which names its config file
set -euo pipefail
# A failure inside a command substitution, such as asked's jq, stops
# the check too, rather than passing an empty value along.
shopt -s inherit_errexit

root=$1
held=$2
start=$3

if [ "$root" != "$held" ]; then
  echo "pi-packages installs into $root, not the folder the fence holds, $held"
  exit 1
fi

config=$(grep -o '/nix/store/[^ ]*-pi-packages.json' <<<"$start" || true)
if [ -z "$config" ]; then
  echo "the service names no pi-packages config in its command: $start"
  exit 1
fi

# Each folder must lie under the root by its words, with no step back up.
inside() {
  local name=$1 value=$2
  case $value in
    */../* | */..) ;;
    "$root"/*) return 0 ;;
    *) ;;
  esac
  echo "pi-packages gives pnpm $name='$value', outside $root, which fenced commands may write"
  exit 1
}

for name in npm_config_store_dir pnpm_config_store_dir npm_config_cache_dir \
  pnpm_config_cache_dir npm_config_devdir; do
  value=$(jq -r --arg name "$name" '.env[$name] // ""' "$config")
  inside "$name" "$value"
done

# pnpm, in a home of its own, given the build's variables and nothing else.
home=$(mktemp -d)
pnpm=$(jq -r .pnpm "$config")
asked() {
  local -a vars=("HOME=$home" "PATH=$(dirname "$pnpm")")
  local pair pairs
  pairs=$(jq -r '.env | to_entries[] | "\(.key)=\(.value)"' "$config")
  while IFS= read -r pair; do
    vars+=("$pair")
  done <<<"$pairs"
  env -i "${vars[@]}" "$pnpm" "$@"
}
store=$(asked store path)
inside "pnpm's store" "$store"
cache=$(asked config get cache-dir)
inside "pnpm's cache" "$cache"
