# shellcheck shell=bash
# Takes Bluetooth's driver off its chip for the length of a hibernation, and
# puts it back after. systemd-sleep runs it with "pre" or "post" and the
# sleep's name.
#
# Going down, the kernel puts every device to sleep, wakes it again to write
# the image, then powers each off. Waking, 7.2's driver resets the chip and
# loads its firmware afresh, and that load can still be waiting when the
# power-off comes: the chip, in its bootloader, never answers, the driver
# refuses, and the kernel rolls the hibernation back while telling systemd
# it worked. A device with no driver has nothing to refuse. The driver's
# fix is mainline's 2926eb396711, which wakes the chip without a reset; this
# goes once the kernel has it and a real hibernation passes.
#
# The devices taken off are kept in a file, so only they are put back, and
# nothing here ever fails the sleep. SYSFS and KEPT move both for the check.
set -u

driver=${SYSFS:-/sys}/bus/pci/drivers/btintel_pcie
kept=${KEPT:-/run/bluetooth-sits-out-hibernation}

case "$2" in
hibernate | hybrid-sleep | suspend-then-hibernate) ;;
*) exit 0 ;;
esac

case "$1" in
pre)
  : >"$kept"
  for device in "$driver"/0000:*; do
    [ -e "$device" ] || continue
    name=${device##*/}
    if echo "$name" >"$driver/unbind"; then
      echo "$name" >>"$kept"
    else
      echo "couldn't take Bluetooth's driver off $name" >&2
    fi
  done
  ;;
post)
  [ -f "$kept" ] || exit 0
  while read -r name; do
    echo "$name" >"$driver/bind" || echo "couldn't give $name its Bluetooth driver back" >&2
  done <"$kept"
  rm -f "$kept"
  ;;
esac
exit 0
