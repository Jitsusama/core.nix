# shellcheck shell=bash
# What ../loader-first.sh, given as the first argument, does at boot, with
# stand-ins for findmnt, lsblk and efibootmgr: it makes the loader an entry
# when none on the ESP starts it, makes one it finds active and first in
# BootOrder, and writes nothing when it already is.
set -euo pipefail

script=$1
scratch=$(mktemp -d)
mkdir "$scratch/bin"
# Each stand-in runs with this shell, since the sandbox has no /usr/bin/env.
stand_in() {
  echo "#!$BASH" >"$1"
  cat >>"$1"
  chmod +x "$1"
}
stand_in "$scratch/bin/findmnt" <<'EOF'
echo /dev/nvme0n1p1
EOF
stand_in "$scratch/bin/lsblk" <<'EOF'
echo "87fd733b-4aa1-4f13-8206-47a8fb8f7cc6 nvme0n1 1"
EOF
# Listing reads the entries; anything else is a write, recorded.
stand_in "$scratch/bin/efibootmgr" <<'EOF'
if [ $# -eq 0 ]; then
  cat "$LISTED"
else
  echo "efibootmgr $*" >>"$RAN"
fi
EOF
export PATH="$scratch/bin:$PATH" LISTED="$scratch/listed" RAN="$scratch/ran"
export ESP=/boot LOADER='\EFI\systemd\systemd-bootx64.efi' LABEL="Linux Boot Manager"

tab=$'\t'
# A Surface's own entries, and a past install's on partitions gone since.
surface="Boot0000* Internal Storage${tab}FvVol(a881d567-6cb0-4eee-8435-2e72d33e45b5)/FvFile(50670071-478f-4be7-ad13-8754f379c62f)53004400440000
Boot0001* USB Storage${tab}FvVol(a881d567-6cb0-4eee-8435-2e72d33e45b5)/FvFile(50670071-478f-4be7-ad13-8754f379c62f)55005300420000
Boot0004* Windows Boot Manager${tab}HD(1,GPT,8a674786-5489-4d0c-89e6-d0801feb5cb6,0x800,0x82000)/\\EFI\\Microsoft\\Boot\\bootmgfw.efi
Boot0005* Linux Boot Manager${tab}HD(3,GPT,02abe2d0-b9b5-404d-8cb1-69c5d4d75793,0x800,0xf3800)/\\EFI\\systemd\\systemd-bootx64.efi"
ours="HD(1,GPT,87fd733b-4aa1-4f13-8206-47a8fb8f7cc6,0x800,0x200000)/\\EFI\\systemd\\systemd-bootx64.efi"

# What the script wrote, given this BootOrder and these entries.
ran_with() {
  : >"$RAN"
  {
    echo "BootCurrent: 0000"
    echo "Timeout: 0 seconds"
    echo "BootOrder: $1"
    echo "$2"
  } >"$LISTED"
  # As writeShellApplication runs it.
  bash -o errexit -o nounset -o pipefail "$script" >/dev/null
  cat "$RAN"
}

expect() {
  local said=$1 order=$2 entries=$3 wanted=$4 got
  got=$(ran_with "$order" "$entries")
  if [ "$got" != "$wanted" ]; then
    echo "$said, expected:" >&2
    echo "${wanted:-nothing written}" >&2
    echo "but it wrote:" >&2
    echo "${got:-nothing}" >&2
    exit 1
  fi
}

# A past install's entry has the loader's name and path, on another
# partition, so it isn't taken for ours.
expect "with only the Surface's entries and a past install's" "0001,0005,0004,0000" "$surface" \
  "efibootmgr --quiet --create --disk /dev/nvme0n1 --part 1 --label Linux Boot Manager --loader \\EFI\\systemd\\systemd-bootx64.efi"
expect "with ours behind the Surface's" "0001,0005,0004,0000,0006" "$surface
Boot0006* Linux Boot Manager${tab}$ours" \
  "efibootmgr --quiet --bootorder 0006,0001,0005,0004,0000"
# The firmware writes the path in whatever case it likes.
expect "with ours, in capitals, outside BootOrder" "0001,0000" "$surface
Boot0007* Linux${tab}${ours^^}" \
  "efibootmgr --quiet --bootorder 0007,0001,0000"
expect "with ours inactive" "0006,0001" "$surface
Boot0006  Linux Boot Manager${tab}$ours" \
  "efibootmgr --quiet --bootnum 0006 --active"
expect "with ours first already" "0006,0001,0000" "$surface
Boot0006* Linux Boot Manager${tab}$ours" ""
echo "the loader is made first in BootOrder, and nothing is written once it is"
