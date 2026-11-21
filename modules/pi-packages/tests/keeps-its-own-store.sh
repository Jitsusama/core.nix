#!/usr/bin/env bash
# Proves pi-packages installs into the folder the agent fence holds
# read-only, and gives pnpm a store inside it under both names pnpm reads,
# never the account's one store, which fenced commands write. pi loads
# every dependency from that store, so one a fenced command could rewrite
# would run outside the fence.
#
# Usage: keeps-its-own-store.sh ROOT HELD EXECSTART
#   ROOT       the root pi-packages is configured with
#   HELD       the folder the fence holds read-only
#   EXECSTART  the service's command, which names its config file
set -euo pipefail

root=$1
held=$2
start=$3

if [ "$root" != "$held" ]; then
  echo "pi-packages installs into $root, not the folder the fence holds, $held"
  exit 1
fi

config=$(grep -o '/nix/store/[^ ]*-pi-packages.json' <<<"$start")

for name in npm_config_store_dir pnpm_config_store_dir; do
  store=$(jq -r --arg name "$name" '.env[$name] // ""' "$config")
  case $store in
    "$root"/*) ;;
    *)
      echo "pi-packages gives pnpm $name='$store', outside $root, which fenced commands may write"
      exit 1
      ;;
  esac
done
