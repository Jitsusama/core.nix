# shellcheck shell=bash
# Forges the house's reports: five a second for 150 s, from `from` to the
# shaper's port at `to`, each saying the counter climbed `bytes` a
# millisecond on the forger's own clock from nought, as a reporter just
# started sends, so the shaper would take them in step were the mac not
# checked. Each is signed with `key`, which isn't the house's. house.nix
# puts it on the house's machine as house-forger, with its tools.
# usage: house-forger.sh KEY BYTES FROM TO
set -euo pipefail

key=$1
bytes=$2
from=$3
to=$4

start=$(date +%s%3N)
for ((sent = 0; sent < 750; sent++)); do
  now=$(date +%s%3N)
  ms=$((now - start))
  text="$((ms * bytes)) ${ms} ${now}"
  mac=$(printf %s "${text}" | openssl dgst -sha256 -hmac "${key}" -r | cut -c1-32)
  printf %s "${text} ${mac}" | socat -u - "UDP:${to}:47110,bind=${from}"
  sleep 0.2
done
