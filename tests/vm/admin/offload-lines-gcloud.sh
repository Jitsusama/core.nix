#!/usr/bin/env bash
# Stands in for gcloud in offload-lines.nix, beside this: describes any instance
# as its maker makes it, running the startup script the machine pins with
# root's own key for its hook, from a disk of an image the machine allows,
# in a project setting only what each builder overrides, with an audit log
# holding its making alone, and says the host key the test gave the cloud;
# but only when the token it's handed can be read, so a token the lines'
# account can't read lends nothing, as gcloud's refusal would.
set -euo pipefail

for argument in "$@"; do
  case $argument in
  --access-token-file=*) test -s "${argument#*=}" ;;
  *) ;;
  esac
done

case "$1 $2 $3" in
"compute instances describe")
  script=$(jq --raw-output .startupScript /etc/nix-offload/lines.json)
  jq --null-input \
    --rawfile script "${script}" \
    --rawfile hook /etc/nix/offload_ed25519.pub \
    '{
      status: "RUNNING",
      creationTimestamp: "2026-11-10T05:14:21.390-08:00",
      lastStartTimestamp: "2026-11-10T05:14:27.724-08:00",
      fingerprint: "Kk2hRYug-uY=",
      metadata: {fingerprint: "MQpHVpNVmsQ=", items: [
        {key: "block-project-ssh-keys", value: "TRUE"},
        {key: "offload-hook-key", value: $hook},
        {key: "startup-script", value: $script}
      ]},
      disks: [{boot: true, source: "https://compute.example/disks/a-builder-1"}]
    }'
  ;;
"compute disks describe")
  jq --null-input '{sourceImage: "https://compute.example/images/a-builder-image"}'
  ;;
"compute project-info describe")
  jq --null-input '{commonInstanceMetadata: {items: [{key: "enable-oslogin", value: "TRUE"}]}}'
  ;;
"logging read "*)
  jq --null-input '[range(2) | {
    timestamp: "2026-11-10T13:14:20Z",
    protoPayload: {
      methodName: "v1.compute.instances.insert",
      resourceName: "projects/a-lab/zones/z/instances/a-builder-1"
    }
  }]'
  ;;
*) cat /etc/offload-lines/host-key ;;
esac
