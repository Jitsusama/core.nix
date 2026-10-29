# shellcheck shell=bash
# What ../firmware-update.sh, given as the first argument, does as the
# machine shuts down, with stand-ins for efibootmgr, systemd-pcrlock and the
# policy's making: it frees PCR 0 only when BootNext names fwupd's boot
# entry, and leaves the policy alone for any other BootNext or none.
set -euo pipefail

script=$1
scratch=$(mktemp -d)
mkdir "$scratch/bin"
# Each stand-in runs with this shell, since the sandbox has no /usr/bin/env.
stand_in() {
  echo "#!$BASH" >"$1"
  cat >>"$1"
}
stand_in "$scratch/bin/efibootmgr" <<'EOF'
cat "$LISTED"
EOF
stand_in "$scratch/pcrlock" <<'EOF'
echo "pcrlock $*" >>"$RAN"
EOF
stand_in "$scratch/make-policy" <<'EOF'
echo make-policy >>"$RAN"
EOF
chmod +x "$scratch/bin/efibootmgr" "$scratch/pcrlock" "$scratch/make-policy"
export PATH="$scratch/bin:$PATH" PCRLOCK="$scratch/pcrlock" MAKE_POLICY="$scratch/make-policy"
export LISTED="$scratch/listed" RAN="$scratch/ran"

tab=$'\t'
entries="Boot0000* UEFI PVC10 SK hynix 1024GB${tab}HD(1,GPT,52c0,0x800,0x200000)/\\EFI\\Boot\\BootX64.efi
Boot0003  Linux Firmware Updater${tab}HD(1,GPT,52c0,0x800,0x200000)/\\EFI\\nixos\\fwupdx64.efi
Boot0004* Linux Firmware Updater${tab}HD(1,GPT,52c0,0x800,0x200000)/\\EFI\\nixos\\fwupdx64.efi
Boot0005* Linux-Firmware-Updater${tab}HD(1,GPT,52c0,0x800,0x200000)/\\EFI\\nixos\\fwupdx64.efi
Boot0006* Not the Linux Firmware Updater${tab}HD(1,GPT,52c0,0x800,0x200000)/\\EFI\\other.efi"

# What the script ran with BootNext as given, none when it's empty.
ran_with() {
  : >"$RAN"
  {
    echo "BootCurrent: 0000"
    echo "Timeout: 0 seconds"
    if [ -n "$1" ]; then
      echo "BootNext: $1"
    fi
    echo "BootOrder: 0000,0004"
    echo "$entries"
  } >"$LISTED"
  # As writeShellApplication runs it.
  bash -o errexit -o nounset -o pipefail "$script" >/dev/null
  cat "$RAN"
}

freed="pcrlock unlock-firmware-code
make-policy"
expect() {
  local next=$1 wanted=$2 got
  got=$(ran_with "$next")
  if [ "$got" != "$wanted" ]; then
    echo "with BootNext '$next', expected:" >&2
    echo "${wanted:-nothing run}" >&2
    echo "but it ran:" >&2
    echo "${got:-nothing}" >&2
    exit 1
  fi
}

expect "" ""
expect 0000 ""
expect 0006 ""
# One BootNext names no entry the firmware lists.
expect 0009 ""
expect 0004 "$freed"
expect 0005 "$freed"
# An entry not in BootOrder still starts as BootNext.
expect 0003 "$freed"
echo "PCR 0 is freed only for fwupd's entry"
