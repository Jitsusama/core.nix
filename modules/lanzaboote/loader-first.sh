# shellcheck shell=bash
# Run at every boot. The firmware starts the first entry in BootOrder that
# starts, measuring each one it tries into PCR 4. lanzaboote writes no
# entry of its own, so a machine boots our loader through one its firmware
# made, after whatever comes before it: on a Surface, its own USB
# application and a past install's entries, which systemd-pcrlock can't
# account for, so PCR 4 left the disk's policy. Our loader gets an entry of
# its own on the ESP, first in BootOrder, and nothing is written when it
# already is.

# The ESP's partition, by the identity the firmware's device path gives it.
source=$(findmnt --noheadings --output SOURCE --target "$ESP")
read -r partuuid disk part < <(lsblk --noheadings --nodeps --output PARTUUID,PKNAME,PARTN "$source")

listed=$(efibootmgr)
# Boot0005* Linux Boot Manager<tab>HD(1,GPT,<partuuid>,0x800,0x200000)/\EFI\systemd\systemd-bootx64.efi
ours=$(grep -i -F "HD($part,GPT,$partuuid," <<<"$listed" | grep -i -F "/$LOADER" | head -n 1 || true)
if [ -z "$ours" ]; then
  echo "no boot entry starts $LOADER on the ESP, so one is made, first in BootOrder"
  efibootmgr --quiet --create --disk "/dev/$disk" --part "$part" --label "$LABEL" --loader "$LOADER"
  exit 0
fi
number=${ours:4:4}
if [ "${ours:8:1}" != "*" ]; then
  echo "Boot$number starts $LOADER but was inactive, so it's made active"
  efibootmgr --quiet --bootnum "$number" --active
fi
order=$(sed -n 's/^BootOrder: //p' <<<"$listed")
if [ "${order%%,*}" = "$number" ]; then
  exit 0
fi
others=$(tr ',' '\n' <<<"$order" | grep -v -x -F "$number" | paste -s -d ',' || true)
echo "Boot$number starts $LOADER, so it goes first in BootOrder, ahead of ${order:-nothing}"
efibootmgr --quiet --bootorder "$number${others:+,$others}"
