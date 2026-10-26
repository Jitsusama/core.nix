#!/usr/bin/env bash
# Seals the machine's Bluetooth pairings to its TPM and puts them on the ESP,
# where the boot stub hands them to the initrd, so a paired keyboard types at
# the disk prompt. Anyone holding a keyboard's link key can type as it, so the
# pairings never sit on the ESP in the clear: they open only where the disk's
# own key would. It seals again only when the pairings changed, so the ESP is
# written once per pairing, not at every run.
#
# The environment names where things go: ESP (the ESP's mount point), POLICY
# (the pcrlock policy the disk's key is bound to, or empty to bind to PCR 7
# alone) and STATE_DIRECTORY (systemd's, for the last seal's checksum).
set -euo pipefail

store=/var/lib/bluetooth
credential="$ESP/loader/credentials/bluetooth-pairing.cred"
sealed="$STATE_DIRECTORY/sealed.sha256"

pairings=$(mktemp)
trap 'rm -f "$pairings" "$credential.new"' EXIT

# Each adapter's settings, each paired device's info file, which holds its
# keys, and that device's cache, which holds its service records: a keyboard's
# HID descriptor is one, and without it the keyboard connects and never types.
# The caches of devices only ever seen in a scan stay out.
listed() {
  local adapter device
  for adapter in */; do
    adapter=${adapter%/}
    [ -f "$adapter/settings" ] && printf '%s\0' "$adapter/settings"
    for device in "$adapter"/*/info; do
      [ -f "$device" ] || continue
      printf '%s\0' "$device"
      device=$(basename "$(dirname "$device")")
      [ -f "$adapter/cache/$device" ] && printf '%s\0' "$adapter/cache/$device"
    done
  done
}
if [ -d "$store" ]; then
  (cd "$store" && listed) |
    tar --create --file="$pairings" --directory="$store" --null --files-from=- \
      --sort=name --mtime=@0 --owner=0 --group=0 --numeric-owner
fi

# Nothing paired leaves nothing on the ESP.
if ! grep -q -a -E '^\[(LinkKey|LongTermKey|PeripheralLongTermKey|IdentityResolvingKey)\]' "$pairings"; then
  rm -f "$credential" "$sealed"
  exit 0
fi

checksum=$(sha256sum "$pairings" | cut -d' ' -f1)
if [ -e "$credential" ] && [ "$(cat "$sealed" 2>/dev/null)" = "$checksum" ]; then
  exit 0
fi

if [ -n "$POLICY" ]; then
  binding=(--tpm2-pcrlock="$POLICY")
else
  binding=(--tpm2-pcrs=7)
fi

mkdir -p "$(dirname "$credential")"
systemd-creds encrypt --name=bluetooth-pairing --with-key=tpm2 "${binding[@]}" \
  "$pairings" "$credential.new"
mv -f "$credential.new" "$credential"
sync --file-system "$credential"
echo "$checksum" >"$sealed"
echo "sealed $(tar --list --file="$pairings" | grep -c '/info$') pairings to $credential"
