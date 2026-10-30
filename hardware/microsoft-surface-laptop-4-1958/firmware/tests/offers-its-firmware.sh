# shellcheck shell=bash
# fwupd, run against this model's firmware as its ESRT shows it, with Surface
# UEFI at 4.355.140 and the Nuvoton TPM at 7.2.2.0, offers the UEFI's update
# from the remote the module writes, as a trusted upgrade, and nothing for
# the TPM. Only fwupd's UEFI capsule plugin loads, and every path it reads
# is a folder here, so it touches no firmware of the machine it runs on.
# usage: offers-its-firmware.sh <remote's .conf> <fwupd> <jq>
set -euo pipefail

remote=$1
fwupd=$2
jq=$3

fail() {
  echo "$*" >&2
  exit 1
}

# An ESRT entry: its GUID, its type (1 the system's firmware, 2 a device's),
# and its version as one number.
entry() {
  local dir=firmware/efi/esrt/entries/$1
  mkdir -p "$dir"
  echo "$2" >"$dir/fw_class"
  echo "$3" >"$dir/fw_type"
  echo "$4" >"$dir/fw_version"
  echo 0 >"$dir/lowest_supported_fw_version"
  echo 0 >"$dir/last_attempt_version"
  echo 0 >"$dir/last_attempt_status"
  echo 0x50000 >"$dir/capsule_flags"
}

entry entry0 6722ccaf-93b4-49c4-a028-24c809b7dd4c 1 $(((4 << 24) | (355 << 8) | 140))
entry entry1 dc2c4b87-4806-4aa2-ad38-5478ea2a01df 2 $(((7 << 24) | (2 << 16) | (2 << 8)))
echo 64 >firmware/efi/fw_platform_size
# fwupd looks for the UEFI variables before it reads the ESRT.
mkdir firmware/efi/efivars

# The machine's own DMI isn't there to read, so its firmware's vendor comes
# from fwupd's settings, as fwupd allows.
mkdir -p etc/fwupd/remotes.d state cache
printf '[fwupd]\nBiosVendor=Microsoft Corporation\n' >etc/fwupd/fwupd.conf
cp "$remote" etc/fwupd/remotes.d/surface-laptop-4-amd.conf

env HOME="$PWD" FWUPD_SYSCONFDIR="$PWD/etc" FWUPD_LOCALSTATEDIR="$PWD/state" \
  CACHE_DIRECTORY="$PWD/cache" FWUPD_SYSFSFWDIR="$PWD/firmware" \
  FWUPD_UEFI_ESP_PATH="$PWD/esp" FWUPD_UEFI_TEST=1 \
  "$fwupd/bin/fwupdtool" --plugins uefi_capsule get-updates --json >updates.json 2>fwupd.log ||
  fail "fwupdtool found no updates: $(cat fwupd.log)"

offered=$("$jq" -r '[.Devices[] | select(.Releases | length > 0)
  | "\(.Guid[0]) \(.Version) \(.Releases[0].Version) \(.Releases[0].Flags | sort | join(","))"]
  | join(";")' updates.json)
want="6722ccaf-93b4-49c4-a028-24c809b7dd4c 4.355.140 4.501.140 is-upgrade,trusted-metadata,trusted-payload"
[ "$offered" = "$want" ] || fail "fwupd offered '$offered', not '$want'"
