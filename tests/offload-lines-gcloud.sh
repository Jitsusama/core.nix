#!/usr/bin/env bash
# Stands in for gcloud in tests/offload-lines.nix: describes any instance
# as its maker makes it, running the startup script the machine pins with
# root's own key for its hook, from a disk of an image the machine allows,
# and says the host key the test gave the cloud; but only when the token
# it's handed can be read, so a token the lines' account can't read lends
# nothing, as gcloud's refusal would.
set -euo pipefail

for argument in "$@"; do
  case $argument in
  --access-token-file=*) test -s "${argument#*=}" ;;
  *) ;;
  esac
done

case "$2 $3" in
"instances describe")
  script=$(jq --raw-output .startupScript /etc/nix-offload/lines.json)
  jq --null-input \
    --rawfile script "${script}" \
    --rawfile hook /etc/nix/offload_ed25519.pub \
    '{
      status: "RUNNING",
      lastStartTimestamp: "2026-11-10T05:14:27.724-08:00",
      metadata: {items: [
        {key: "block-project-ssh-keys", value: "TRUE"},
        {key: "offload-hook-key", value: $hook},
        {key: "startup-script", value: $script}
      ]},
      disks: [{boot: true, source: "https://compute.example/disks/a-builder-1"}]
    }'
  ;;
"disks describe")
  jq --null-input '{sourceImage: "https://compute.example/images/a-builder-image"}'
  ;;
*) cat /etc/offload-lines/host-key ;;
esac
