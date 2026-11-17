#!/usr/bin/env bash
# Sees the tools every machine's kit shares in the kit at $1, its curl
# speaking wss://, so a tool one system lacks fails on that system's checks
# before a machine of it switches.
set -euo pipefail

kit=$1
for tool in curl fd jq mycelium rg websocat; do
  [[ -x "$kit/bin/$tool" ]] || {
    echo "the kit at $kit lacks $tool"
    exit 1
  }
done
said=$("$kit/bin/curl" --version)
[[ $said == *" wss"* ]] || {
  echo "the kit's curl doesn't speak wss://: $said"
  exit 1
}
