#!/usr/bin/env bash
# Stands in for gcloud in tests/offload-lines.nix: says the host key the
# test gave the cloud for any instance, but only when the token it's
# handed can be read, so a token the lines' account can't read lends
# nothing, as gcloud's refusal would.
set -euo pipefail

for argument in "$@"; do
  case $argument in
  --access-token-file=*) test -s "${argument#*=}" ;;
  esac
done
cat /etc/offload-lines/host-key
